#![no_std]

use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype,
    token,
    Address, Env, String,
};

// ─────────────────────────────────────────────────────────────────────────────
// Storage keys — every persisted entry is namespaced here
// ─────────────────────────────────────────────────────────────────────────────
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Task(u64),  // task_id → TaskRecord
    TaskCount,  // global auto-increment counter
}

// ─────────────────────────────────────────────────────────────────────────────
// Task status state machine
// ─────────────────────────────────────────────────────────────────────────────
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum TaskStatus {
    Open,       // funded, awaiting a worker
    Assigned,   // worker accepted, in progress
    Submitted,  // worker submitted proof of completion
    Completed,  // manager approved → USDC released to worker
    Cancelled,  // manager cancelled before assignment → USDC refunded
    Disputed,   // manager rejected submission → USDC clawed back
}

// ─────────────────────────────────────────────────────────────────────────────
// Core task record — persisted in contract storage per task_id
// ─────────────────────────────────────────────────────────────────────────────
#[contracttype]
#[derive(Clone)]
pub struct TaskRecord {
    pub task_id:         u64,
    pub manager:         Address,         // created & funded the task
    pub worker:          Option<Address>, // assigned worker (None until accepted)
    pub usdc_token:      Address,         // USDC token contract address
    pub bounty:          i128,            // bounty in USDC stroops (1 USDC = 10_000_000)
    pub description:     String,          // human-readable task description
    pub completion_hash: Option<String>,  // worker-submitted proof hash
    pub status:          TaskStatus,
    pub deadline_ledger: u32,            // task expires at this ledger sequence
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract events — defined with #[contractevent] (replaces deprecated publish)
// Each struct represents one emitted event type; fields become the event data.
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when a manager creates and funds a new task.
#[contractevent]
pub struct TaskCreated {
    pub task_id: u64,
    pub manager: Address,
    pub bounty:  i128,
}

/// Emitted when a worker accepts an open task.
#[contractevent]
pub struct TaskAccepted {
    pub task_id: u64,
    pub worker:  Address,
}

/// Emitted when a worker submits their completion proof hash.
#[contractevent]
pub struct TaskSubmitted {
    pub task_id:         u64,
    pub completion_hash: String,
}

/// Emitted when a manager approves and USDC is released to the worker.
#[contractevent]
pub struct TaskReleased {
    pub task_id: u64,
    pub worker:  Address,
    pub bounty:  i128,
}

/// Emitted when a manager rejects submitted work and USDC is clawed back.
#[contractevent]
pub struct TaskDisputed {
    pub task_id: u64,
    pub manager: Address,
    pub bounty:  i128,
}

/// Emitted when a manager cancels an open task and is refunded.
#[contractevent]
pub struct TaskCancelled {
    pub task_id: u64,
    pub manager: Address,
    pub bounty:  i128,
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract entrypoint
// ─────────────────────────────────────────────────────────────────────────────
#[contract]
pub struct TaskPayContract;

#[contractimpl]
impl TaskPayContract {

    // ── 1. CREATE TASK ────────────────────────────────────────────────────────
    // Manager locks USDC into escrow and registers a task on-chain.
    // Returns the auto-assigned task_id.
    pub fn create_task(
        env:             Env,
        manager:         Address,
        usdc_token:      Address,
        bounty:          i128,
        description:     String,
        deadline_ledger: u32,
    ) -> u64 {
        // Manager must sign this transaction
        manager.require_auth();

        if bounty <= 0 {
            panic!("bounty must be greater than zero");
        }

        // Lock USDC from manager wallet into this contract (escrow)
        let token_client = token::Client::new(&env, &usdc_token);
        token_client.transfer(
            &manager,
            &env.current_contract_address(),
            &bounty,
        );

        // Auto-increment the global task counter
        let task_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0u64)
            + 1;
        env.storage().instance().set(&DataKey::TaskCount, &task_id);

        // Persist the new task record
        let task = TaskRecord {
            task_id,
            manager: manager.clone(),
            worker: None,
            usdc_token,
            bounty,
            description,
            completion_hash: None,
            status: TaskStatus::Open,
            deadline_ledger,
        };
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        // Emit structured event (replaces deprecated events().publish())
        env.events().publish_event(&TaskCreated { task_id, manager, bounty });

        task_id
    }

    // ── 2. ACCEPT TASK ────────────────────────────────────────────────────────
    // Worker claims an open task, recording their address on-chain.
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

        // Reject stale tasks past their deadline
        if env.ledger().sequence() > task.deadline_ledger {
            panic!("task deadline has passed");
        }

        task.worker = Some(worker.clone());
        task.status = TaskStatus::Assigned;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish_event(&TaskAccepted { task_id, worker });
    }

    // ── 3. SUBMIT COMPLETION ──────────────────────────────────────────────────
    // Worker submits a proof-of-completion hash (e.g. SHA-256 of a PR URL).
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

        // Only the assigned worker may submit
        let worker = task.worker.clone().expect("no worker assigned");
        worker.require_auth();

        if task.status != TaskStatus::Assigned {
            panic!("task is not in assigned state");
        }

        task.completion_hash = Some(completion_hash.clone());
        task.status = TaskStatus::Submitted;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish_event(&TaskSubmitted { task_id, completion_hash });
    }

    // ── 4. APPROVE AND RELEASE ────────────────────────────────────────────────
    // Manager approves submitted work — escrowed USDC is transferred to worker.
    pub fn approve_and_release(
        env:     Env,
        task_id: u64,
    ) {
        let mut task: TaskRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found");

        // Only the original manager may approve
        task.manager.require_auth();

        if task.status != TaskStatus::Submitted {
            panic!("task has not been submitted for review");
        }

        let worker = task.worker.clone().expect("no worker on task");

        // Release escrowed USDC → worker wallet
        let token_client = token::Client::new(&env, &task.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &worker,
            &task.bounty,
        );

        let bounty = task.bounty;
        let manager = task.manager.clone();
        task.status = TaskStatus::Completed;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish_event(&TaskReleased { task_id, worker, bounty });
        let _ = manager; // suppress unused warning
    }

    // ── 5. REJECT SUBMISSION ──────────────────────────────────────────────────
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

        // Clawback: return USDC to manager
        let token_client = token::Client::new(&env, &task.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &task.manager,
            &task.bounty,
        );

        let bounty  = task.bounty;
        let manager = task.manager.clone();
        task.status = TaskStatus::Disputed;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish_event(&TaskDisputed { task_id, manager, bounty });
    }

    // ── 6. CANCEL TASK ────────────────────────────────────────────────────────
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

        // Can only cancel before a worker has accepted
        if task.status != TaskStatus::Open {
            panic!("can only cancel open tasks");
        }

        let token_client = token::Client::new(&env, &task.usdc_token);
        token_client.transfer(
            &env.current_contract_address(),
            &task.manager,
            &task.bounty,
        );

        let bounty  = task.bounty;
        let manager = task.manager.clone();
        task.status = TaskStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Task(task_id), &task);

        env.events().publish_event(&TaskCancelled { task_id, manager, bounty });
    }

    // ── 7. GET TASK (read-only) ───────────────────────────────────────────────
    // Returns the full TaskRecord for off-chain consumers and frontend UIs.
    pub fn get_task(env: Env, task_id: u64) -> TaskRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Task(task_id))
            .expect("task not found")
    }

    // ── 8. GET TASK COUNT (read-only) ─────────────────────────────────────────
    // Returns total number of tasks created (used to verify deployment).
    pub fn get_task_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0u64)
    }
}