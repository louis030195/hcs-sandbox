//! End-to-end integration tests
//! Run with: cargo test --test e2e -- --ignored (requires running hvkube serve)

use std::time::Duration;

const API_URL: &str = "http://localhost:8080";

#[derive(Debug, serde::Deserialize)]
struct HealthResponse {
    status: String,
}

#[derive(Debug, serde::Deserialize)]
struct ResumeResponse {
    vm_id: String,
    vm_name: String,
    ip_address: String,
    mcp_endpoint: String,
    resume_time_ms: u64,
}

#[derive(Debug, serde::Serialize)]
struct AcquireRequest {
    pool_name: String,
}

#[derive(Debug, serde::Serialize)]
struct ReleaseRequest {
    reset: bool,
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
}

#[test]
#[ignore] // Run manually: cargo test --test e2e -- --ignored
fn test_health() {
    let resp: HealthResponse = client()
        .get(format!("{}/health", API_URL))
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    assert_eq!(resp.status, "ok");
}

#[test]
#[ignore]
fn test_acquire_and_release() {
    let c = client();
    
    // Acquire VM
    let vm: ResumeResponse = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "agents".to_string() })
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    println!("Acquired {} @ {} in {}ms", vm.vm_name, vm.ip_address, vm.resume_time_ms);
    assert!(!vm.ip_address.is_empty());
    assert!(vm.mcp_endpoint.contains(":8080/mcp"));
    
    // Check terminator health
    let health: HealthResponse = c
        .get(format!("http://{}:8080/health", vm.ip_address))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(health.status, "healthy");
    
    // Release VM
    let resp = c
        .post(format!("{}/api/v1/vms/{}/release", API_URL, vm.vm_name))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
    assert!(resp.status().is_success());
}

#[test]
#[ignore]
fn test_parallel_acquire() {
    use std::thread;
    
    let handles: Vec<_> = (0..2)
        .map(|i| {
            thread::spawn(move || {
                let c = client();
                let start = std::time::Instant::now();
                
                let vm: ResumeResponse = c
                    .post(format!("{}/api/v1/acquire", API_URL))
                    .json(&AcquireRequest { pool_name: "agents".to_string() })
                    .send()
                    .unwrap()
                    .json()
                    .unwrap();
                
                println!("[{}] Acquired {} in {}ms", i, vm.vm_name, start.elapsed().as_millis());
                vm
            })
        })
        .collect();
    
    let vms: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    
    // All VMs should be different
    assert_ne!(vms[0].vm_id, vms[1].vm_id);
    
    // Release all
    let c = client();
    for vm in vms {
        c.post(format!("{}/api/v1/vms/{}/release", API_URL, vm.vm_name))
            .json(&ReleaseRequest { reset: false })
            .send()
            .unwrap();
    }
}

#[test]
#[ignore]
fn test_mcp_tools_list() {
    let c = client();
    
    // Acquire
    let vm: ResumeResponse = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "agents".to_string() })
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    // Call MCP tools/list
    let mcp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });
    
    let mcp_resp: serde_json::Value = c
        .post(&vm.mcp_endpoint)
        .json(&mcp_req)
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    let tools = mcp_resp["result"]["tools"].as_array().unwrap();
    println!("MCP agent has {} tools", tools.len());
    assert!(!tools.is_empty());
    
    // Release
    c.post(format!("{}/api/v1/vms/{}/release", API_URL, vm.vm_name))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
}

// === Error Case Tests ===

#[test]
#[ignore]
fn test_acquire_from_nonexistent_pool() {
    let c = client();
    
    let resp = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "nonexistent-pool".to_string() })
        .send()
        .unwrap();
    
    assert_eq!(resp.status(), 404);
}

#[test]
#[ignore]
fn test_release_nonexistent_vm() {
    let c = client();
    
    let resp = c
        .post(format!("{}/api/v1/vms/nonexistent-vm/release", API_URL))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
    
    assert_eq!(resp.status(), 404);
}

#[test]
#[ignore]
fn test_resume_nonexistent_vm() {
    let c = client();
    
    let resp = c
        .post(format!("{}/api/v1/vms/nonexistent-vm/resume", API_URL))
        .send()
        .unwrap();
    
    assert_eq!(resp.status(), 404);
}

#[test]
#[ignore]
fn test_get_nonexistent_template() {
    let c = client();
    
    let resp = c
        .get(format!("{}/api/v1/templates/nonexistent", API_URL))
        .send()
        .unwrap();
    
    assert_eq!(resp.status(), 404);
}

#[test]
#[ignore]
fn test_double_acquire_same_vm() {
    let c = client();
    
    // Acquire first VM
    let vm1: ResumeResponse = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "agents".to_string() })
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    // Try to acquire again - should get different VM
    let vm2: ResumeResponse = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "agents".to_string() })
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    // Should be different VMs
    assert_ne!(vm1.vm_id, vm2.vm_id);
    
    // Release both
    c.post(format!("{}/api/v1/vms/{}/release", API_URL, vm1.vm_name))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
    c.post(format!("{}/api/v1/vms/{}/release", API_URL, vm2.vm_name))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
}

#[test]
#[ignore]
fn test_acquire_release_cycle() {
    let c = client();
    
    // Acquire
    let vm: ResumeResponse = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "agents".to_string() })
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    let vm_name = vm.vm_name.clone();
    
    // Release
    c.post(format!("{}/api/v1/vms/{}/release", API_URL, vm_name))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
    
    // Acquire again - might get same VM back
    let vm2: ResumeResponse = c
        .post(format!("{}/api/v1/acquire", API_URL))
        .json(&AcquireRequest { pool_name: "agents".to_string() })
        .send()
        .unwrap()
        .json()
        .unwrap();
    
    // Release
    c.post(format!("{}/api/v1/vms/{}/release", API_URL, vm2.vm_name))
        .json(&ReleaseRequest { reset: false })
        .send()
        .unwrap();
}
