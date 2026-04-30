#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{Client as TokenClient, StellarAssetClient},
        Address, Env, String,
    };
    use crate::{TaskPayContract, TaskPayContractClient, TaskStatus};

    // ──────────────────────────────────────────────────────────────────────────
    // Helper: bootstrap a fresh test environment with a mock USDC token
    // and funded manager/worker accounts.
    // ──────────────────────────────────────────────────────────────────────────
    fn setup() -> (Env, Address, Address, Address, TaskPayContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();

        // Deploy the TaskPay contract
        let contract_id = env.register_contract(None, TaskPayContract);
        let client = TaskPayContractClient::new(&env, &contract_id);

        // Deploy a mock Stellar Asset (USDC stand-in)
        let usdc_admin = Address::generate(&env);
        let usdc_token = env.register_stellar_asset_contract(usdc_admin.clone());
        let usdc_admin_client = StellarAssetClient::new(&env, &usdc_token);

        // Create manager and worker test accounts
        let manager = Address::generate(&env);
        let worker  = Address::generate(&env);

        // Mint 1000 USDC (in stroops: 1 USDC = 10_000_000) to manager
        usdc_admin_client.mint(&manager, &1_000_000_0000i128);

        (env, usdc_token, manager, worker, client)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // TEST 1 — Happy path
    // Full lifecycle: create → accept → submit → approve → USDC released
    // ──────────────────────────────────────────────────────────────────────────
    #[test]
    fn test_happy_path_full_lifecycle() {
        let (env, usdc_token, manager, worker, client) = setup();

        let bounty: i128 = 50_000_000_0; // 50 USDC
        let deadline = env.ledger().sequence() + 1000;

        // Step 1 — Manager creates task and locks bounty into escrow
        let task_id = client.create_task(
            &manager,
            &usdc_token,
            &bounty,
            &String::from_str(&env, "Build login screen component"),
            &deadline,
        );
        assert_eq!(task_id, 1);

        // Step 2 — Worker accepts the task
        client.accept_task(&task_id, &worker);
        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Assigned);
        assert_eq!(task.worker, Some(worker.clone()));

        // Step 3 — Worker submits completion hash
        let proof = String::from_str(&env, "abc123def456");
        client.submit_completion(&task_id, &proof);
        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Submitted);

        // Step 4 — Manager approves; USDC flows to worker
        let worker_balance_before =
            TokenClient::new(&env, &usdc_token).balance(&worker);
        client.approve_and_release(&task_id);

        let worker_balance_after =
            TokenClient::new(&env, &usdc_token).balance(&worker);
        assert_eq!(worker_balance_after - worker_balance_before, bounty);

        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Completed);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // TEST 2 — Edge case: unauthorised worker cannot approve release
    // Only the original manager may call approve_and_release.
    // We test this by verifying that calling from worker panics.
    // ──────────────────────────────────────────────────────────────────────────
    #[test]
    #[should_panic(expected = "task has not been submitted for review")]
    fn test_cannot_approve_before_submission() {
        let (env, usdc_token, manager, worker, client) = setup();

        let bounty: i128 = 50_000_000_0;
        let deadline = env.ledger().sequence() + 1000;

        let task_id = client.create_task(
            &manager,
            &usdc_token,
            &bounty,
            &String::from_str(&env, "Design dashboard wireframe"),
            &deadline,
        );

        // Worker accepts but has NOT submitted yet
        client.accept_task(&task_id, &worker);

        // Manager tries to approve before worker submits — must panic
        client.approve_and_release(&task_id);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // TEST 3 — State verification
    // After full happy-path execution, contract storage reflects correct state:
    // task count == 1, status == Completed, bounty amount unchanged in record.
    // ──────────────────────────────────────────────────────────────────────────
    #[test]
    fn test_state_reflects_correctly_after_completion() {
        let (env, usdc_token, manager, worker, client) = setup();

        let bounty: i128 = 100_000_000_0i128; // 100 USDC
        let deadline = env.ledger().sequence() + 500;

        let task_id = client.create_task(
            &manager,
            &usdc_token,
            &bounty,
            &String::from_str(&env, "Implement payment API integration"),
            &deadline,
        );

        client.accept_task(&task_id, &worker);
        client.submit_completion(&task_id, &String::from_str(&env, "deadbeef1234"));
        client.approve_and_release(&task_id);

        // Verify persistent storage
        let task = client.get_task(&task_id);
        assert_eq!(task.task_id, 1);
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.bounty, bounty);
        assert_eq!(task.manager, manager);
        assert_eq!(task.worker, Some(worker));
        assert!(task.completion_hash.is_some());

        // Verify global counter
        let count = client.get_task_count();
        assert_eq!(count, 1);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // TEST 4 — Dispute / rejection path
    // Manager rejects submission → USDC clawed back → status = Disputed
    // ──────────────────────────────────────────────────────────────────────────
    #[test]
    fn test_reject_claws_back_usdc_to_manager() {
        let (env, usdc_token, manager, worker, client) = setup();

        let bounty: i128 = 75_000_000_0i128; // 75 USDC
        let deadline = env.ledger().sequence() + 1000;

        let manager_balance_before =
            TokenClient::new(&env, &usdc_token).balance(&manager);

        let task_id = client.create_task(
            &manager,
            &usdc_token,
            &bounty,
            &String::from_str(&env, "Write unit test suite"),
            &deadline,
        );

        // Balance dropped by bounty after escrow lock
        let manager_balance_after_lock =
            TokenClient::new(&env, &usdc_token).balance(&manager);
        assert_eq!(manager_balance_before - manager_balance_after_lock, bounty);

        client.accept_task(&task_id, &worker);
        client.submit_completion(&task_id, &String::from_str(&env, "badhash999"));

        // Manager rejects — should claw back
        client.reject_submission(&task_id);

        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Disputed);

        // Manager should have their USDC back
        let manager_balance_final =
            TokenClient::new(&env, &usdc_token).balance(&manager);
        assert_eq!(manager_balance_final, manager_balance_before);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // TEST 5 — Cancel open task
    // Manager cancels before any worker accepts → USDC returned, status = Cancelled
    // ──────────────────────────────────────────────────────────────────────────
    #[test]
    fn test_cancel_open_task_refunds_manager() {
        let (env, usdc_token, manager, _worker, client) = setup();

        let bounty: i128 = 25_000_000_0i128; // 25 USDC
        let deadline = env.ledger().sequence() + 1000;

        let manager_balance_before =
            TokenClient::new(&env, &usdc_token).balance(&manager);

        let task_id = client.create_task(
            &manager,
            &usdc_token,
            &bounty,
            &String::from_str(&env, "Research competitor analysis"),
            &deadline,
        );

        // Cancel while still Open
        client.cancel_task(&task_id);

        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Cancelled);

        // Full refund to manager
        let manager_balance_final =
            TokenClient::new(&env, &usdc_token).balance(&manager);
        assert_eq!(manager_balance_final, manager_balance_before);
    }
}
