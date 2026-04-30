#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    token, symbol_short,
    Address, Env, String,
};

// ─────────────────────────────────────────────
// Storage key enum — every persisted key lives here
// ─────────────────────────────────────────────
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Task(u64),      // task_id → TaskRecord
    TaskCount,      // global counter so each task gets a unique id
}

// ─────────────────────────────────────────────
// Task status lifecycle
// ─────────────────────────────────────────────
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum TaskStatus {
    Open,       // funded, awaiting a worker
    Assigned,   // worker accepted, in progress
    Submitted,  // worker submitted proof of completion
    Completed,  // manager approved → USDC released to worker
    Cancelled,  // manager cancelled before assignment → USDC clawed back
    Disputed,   // manager rejected submission → USDC clawed back to manager
}

// ─────────────────────────────────────────────
// Core data struct stored per task
// ─────────────────────────────────────────────
#[contracttype]
#[derive(Clone)]
pub struct TaskRecord {
    pub task_id:          u64,
    pub manager:          Address,          // created & funded the task
    pub worker:           Option<Address>,  // assigned worker (None until accepted)
    pub usdc_token:       Address,          // USDC token contract address on Stellar
    pub bounty:           i128,             // amount in USDC stroops (1 USDC = 10_000_000)
    pub description:      String,           // human-readable task description
    pub completion_hash:  Option<String>,   // worker-submitted proof (e.g. hashed PR URL)
    pub status:           TaskStatus,
    pub deadline_ledger:  u32,             // absolute ledger number for expiry
}

// ─────────────────────────────────────────────
// Contract entrypoint
// ─────────────────────────────────────────────
#[contract]
pub struct TaskPayContract;

#[contractimpl]
impl TaskPayContract {

    // 1. CREATE TASK
    // Manager calls this to lock USDC into escrow and register a new task.
    // Returns the new task_id.
    pub fn create_task(
        env:            Env,
        manager:        Address,
        usdc_token:     Address,
        bounty:         i128,
        description:    String,
        deadline_ledger: u32,
    ) -> u64 {
        manager.require_auth();

        if bounty <= 0 {
            panic!("bounty must be greater than zero");
        }

        // Transfer USDC from manager wallet into this contract (escrow)
        let token_client = token::Client::new(&env, &usdc_token);
        token_client.transfer(
            &manager,
            &env.current_contract_address(),
            &bounty,
        );

        // Auto-incrementing task counter
        let task_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0u64)
            + 1;
        env.storage().instance().set(&DataKey::TaskCount, &task_id);

        let task = TaskRecord {
            task_id,
            manager,
            worker: None,
            usdc_token,
            bounty,
            description,
            completion_hash: None,
            status: TaskStatus::Open,
            deadline_ledger,
        };
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish(
            (symbol_short!("created"), task_id),
            bounty,
        );

        task_id
    }

    // 2. ACCEPT TASK
    // Worker claims an open task.
    pub fn accept_task(
        env:     Env,
        task_id: u64,
        worker:  Address,
    ) {
        worker.require_auth();

        let mut task: TaskRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found");

        if task.status != TaskStatus::Open {
            panic!("task is not open");
        }

        if env.ledger().sequence() > task.deadline_ledger {
            panic!("task deadline has passed");
        }

        task.worker = Some(worker.clone());
        task.status = TaskStatus::Assigned;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish(
            (symbol_short!("accepted"), task_id),
            worker,
        );
    }

    // 3. SUBMIT COMPLETION
    // Worker submits proof-of-completion hash (e.g. SHA-256 of a PR URL).
    pub fn submit_completion(
        env:             Env,
        task_id:         u64,
        completion_hash: String,
    ) {
        let mut task: TaskRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found");

        let worker = task.worker.clone().expect("no worker assigned");
        worker.require_auth();

        if task.status != TaskStatus::Assigned {
            panic!("task is not in assigned state");
        }

        task.completion_hash = Some(completion_hash.clone());
        task.status = TaskStatus::Submitted;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish(
            (symbol_short!("submitted"), task_id),
            completion_hash,
        );
    }

    // 4. APPROVE AND RELEASE
    // Manager approves submitted work and releases escrowed USDC to worker.
    pub fn approve_and_release(
        env:     Env,
        task_id: u64,
    ) {
        let mut task: TaskRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found");

        task.manager.require_auth();

        if task.status != TaskStatus::Submitted {
            panic!("task has not been submitted for review");
        }

        let worker = task.worker.clone().expect("no worker on task");

        // Transfer escrowed USDC → worker wallet
        let token_client = token::Client::new(&env, &task.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &worker,
            &task.bounty,
        );

        task.status = TaskStatus::Completed;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish(
            (symbol_short!("released"), task_id),
            task.bounty,
        );
    }

    // 5. REJECT SUBMISSION
    // Manager rejects submitted work — USDC is clawed back to manager.
    pub fn reject_submission(
        env:     Env,
        task_id: u64,
    ) {
        let mut task: TaskRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found");

        task.manager.require_auth();

        if task.status != TaskStatus::Submitted {
            panic!("task has not been submitted for review");
        }

        // Clawback USDC to manager
        let token_client = token::Client::new(&env, &task.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &task.manager,
            &task.bounty,
        );

        task.status = TaskStatus::Disputed;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish(
            (symbol_short!("disputed"), task_id),
            task.bounty,
        );
    }

    // 6. CANCEL TASK
    // Manager cancels an open (unassigned) task and reclaims USDC.
    pub fn cancel_task(
        env:     Env,
        task_id: u64,
    ) {
        let mut task: TaskRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found");

        task.manager.require_auth();

        if task.status != TaskStatus::Open {
            panic!("can only cancel open tasks");
        }

        let token_client = token::Client::new(&env, &task.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &task.manager,
            &task.bounty,
        );

        task.status = TaskStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish(
            (symbol_short!("cancelled"), task_id),
            task.bounty,
        );
    }

    // 7. GET TASK (read-only)
    pub fn get_task(env: Env, task_id: u64) -> TaskRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found")
    }

    // 8. GET TASK COUNT (read-only)
    pub fn get_task_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0u64)
    }
}
