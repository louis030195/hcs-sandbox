//! API request handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::models::*;
use crate::Orchestrator;
use super::types::*;

pub type AppState = Arc<Orchestrator>;

// === Health ===

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// === Templates ===

pub async fn list_templates(
    State(orch): State<AppState>,
) -> Result<Json<Vec<TemplateResponse>>, (StatusCode, Json<ApiError>)> {
    let templates = orch.list_templates().map_err(to_api_error)?;
    Ok(Json(templates.into_iter().map(template_to_response).collect()))
}

pub async fn create_template(
    State(orch): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), (StatusCode, Json<ApiError>)> {
    let template = Template::new(&req.name, &req.vhdx_path)
        .with_memory(req.memory_mb)
        .with_cpus(req.cpu_count)
        .with_gpu(req.gpu_enabled);

    let template_clone = Template {
        id: template.id.clone(),
        name: template.name.clone(),
        vhdx_path: template.vhdx_path.clone(),
        memory_mb: template.memory_mb,
        cpu_count: template.cpu_count,
        gpu_enabled: template.gpu_enabled,
        installed_software: template.installed_software.clone(),
        created_at: template.created_at,
        description: req.description.clone(),
    };

    orch.register_template(template).map_err(to_api_error)?;
    Ok((StatusCode::CREATED, Json(template_to_response(template_clone))))
}

pub async fn get_template(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<TemplateResponse>, (StatusCode, Json<ApiError>)> {
    let template = orch.get_template(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Template"))?;
    Ok(Json(template_to_response(template)))
}

pub async fn delete_template(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let template = orch.get_template(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Template"))?;
    orch.db().delete_template(&template.id).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("Template '{}' deleted", name) }))
}

// === Pools ===

pub async fn list_pools(
    State(orch): State<AppState>,
) -> Result<Json<Vec<PoolResponse>>, (StatusCode, Json<ApiError>)> {
    let pools = orch.list_pools().map_err(to_api_error)?;
    Ok(Json(pools.into_iter().map(pool_to_response).collect()))
}

pub async fn create_pool(
    State(orch): State<AppState>,
    Json(req): Json<CreatePoolRequest>,
) -> Result<(StatusCode, Json<PoolResponse>), (StatusCode, Json<ApiError>)> {
    let template = orch.get_template(&req.template_name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Template"))?;

    let pool = VMPool::new(&req.name, &template.id)
        .with_count(req.desired_count)
        .with_warm_count(req.warm_count);

    let pool_clone = VMPool {
        id: pool.id.clone(),
        name: pool.name.clone(),
        template_id: pool.template_id.clone(),
        desired_count: pool.desired_count,
        warm_count: pool.warm_count,
        max_per_host: pool.max_per_host,
        created_at: pool.created_at,
    };

    orch.create_pool(pool).map_err(to_api_error)?;
    Ok((StatusCode::CREATED, Json(pool_to_response(pool_clone))))
}

pub async fn get_pool(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PoolStatusResponse>, (StatusCode, Json<ApiError>)> {
    let pool = orch.db().get_pool_by_name(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Pool"))?;
    let status = orch.get_pool_status(&pool.id).map_err(to_api_error)?;
    Ok(Json(PoolStatusResponse {
        id: status.id,
        name: status.name,
        template_id: status.template_id,
        desired_count: status.desired_count,
        total_vms: status.total_vms,
        running_vms: status.running_vms,
        saved_vms: status.saved_vms,
        off_vms: status.off_vms,
        error_vms: status.error_vms,
    }))
}

pub async fn provision_pool(
    State(orch): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ProvisionRequest>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<ApiError>)> {
    let pool = orch.db().get_pool_by_name(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Pool"))?;
    let ids = orch.provision_pool(&pool.id, req.count).map_err(to_api_error)?;
    Ok(Json(ids))
}

pub async fn prepare_pool(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let pool = orch.db().get_pool_by_name(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Pool"))?;

    let vms = orch.db().list_vms_by_pool(&pool.id).map_err(to_api_error)?;
    let mut prepared = 0;

    for vm in vms.iter().filter(|v| v.state == VMState::Off) {
        orch.prepare_vm(&vm.id).map_err(to_api_error)?;
        prepared += 1;
    }

    Ok(Json(ApiSuccess { message: format!("Prepared {} VMs", prepared) }))
}

pub async fn delete_pool(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let pool = orch.db().get_pool_by_name(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Pool"))?;
    orch.db().delete_pool(&pool.id).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("Pool '{}' deleted", name) }))
}

// === VMs ===

pub async fn list_vms(
    State(orch): State<AppState>,
) -> Result<Json<Vec<VMResponse>>, (StatusCode, Json<ApiError>)> {
    let vms = orch.list_vms().map_err(to_api_error)?;
    Ok(Json(vms.into_iter().map(vm_to_response).collect()))
}

pub async fn get_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<VMResponse>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    Ok(Json(vm_to_response(vm)))
}

pub async fn resume_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ResumeResponse>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;

    let start = std::time::Instant::now();
    let ip = orch.resume_vm(&vm.id).map_err(to_api_error)?;
    let elapsed = start.elapsed();

    Ok(Json(ResumeResponse {
        vm_id: vm.id,
        vm_name: vm.name,
        ip_address: ip.clone(),
        mcp_endpoint: format!("http://{}:8080/mcp", ip),
        resume_time_ms: elapsed.as_millis() as u64,
    }))
}

pub async fn save_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    orch.save_vm(&vm.id).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("VM '{}' saved", name) }))
}

pub async fn reset_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    orch.reset_vm(&vm.id).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("VM '{}' reset to clean checkpoint", name) }))
}

pub async fn stop_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    orch.stop_vm(&vm.id, true).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("VM '{}' stopped", name) }))
}

pub async fn delete_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    orch.delete_vm(&vm.id).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("VM '{}' deleted", name) }))
}

pub async fn prepare_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    orch.prepare_vm(&vm.id).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("VM '{}' prepared", name) }))
}

// === Acquire/Release ===

pub async fn acquire_vm(
    State(orch): State<AppState>,
    Json(req): Json<AcquireVMRequest>,
) -> Result<Json<ResumeResponse>, (StatusCode, Json<ApiError>)> {
    let pool = orch.db().get_pool_by_name(&req.pool_name).map_err(to_api_error)?
        .ok_or_else(|| not_found("Pool"))?;

    let start = std::time::Instant::now();
    let vm = orch.acquire_vm(&pool.id).map_err(to_api_error)?;
    let elapsed = start.elapsed();

    Ok(Json(ResumeResponse {
        vm_id: vm.id,
        vm_name: vm.name,
        ip_address: vm.ip_address.clone().unwrap_or_default(),
        mcp_endpoint: format!("http://{}:8080/mcp", vm.ip_address.as_deref().unwrap_or("0.0.0.0")),
        resume_time_ms: elapsed.as_millis() as u64,
    }))
}

pub async fn release_vm(
    State(orch): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ReleaseVMRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let vm = orch.get_vm(&name).map_err(to_api_error)?
        .ok_or_else(|| not_found("VM"))?;
    orch.release_vm(&vm.id, req.reset).map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: format!("VM '{}' released", name) }))
}

// === Reconcile ===

pub async fn reconcile(
    State(orch): State<AppState>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    orch.reconcile().map_err(to_api_error)?;
    Ok(Json(ApiSuccess { message: "Reconciled state with Hyper-V".to_string() }))
}

// === Helpers ===

fn to_api_error(e: crate::Error) -> (StatusCode, Json<ApiError>) {
    let status = match &e {
        crate::Error::VMNotFound(_) => StatusCode::NOT_FOUND,
        crate::Error::TemplateNotFound(_) => StatusCode::NOT_FOUND,
        crate::Error::PoolNotFound(_) => StatusCode::NOT_FOUND,
        crate::Error::NoVMAvailable => StatusCode::SERVICE_UNAVAILABLE,
        crate::Error::InvalidState { .. } => StatusCode::CONFLICT,
        crate::Error::Timeout => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (status, Json(ApiError {
        error: format!("{:?}", e).split('(').next().unwrap_or("Error").to_string(),
        message: e.to_string(),
    }))
}

fn not_found(resource: &str) -> (StatusCode, Json<ApiError>) {
    (StatusCode::NOT_FOUND, Json(ApiError {
        error: "NotFound".to_string(),
        message: format!("{} not found", resource),
    }))
}

fn template_to_response(t: Template) -> TemplateResponse {
    TemplateResponse {
        id: t.id,
        name: t.name,
        vhdx_path: t.vhdx_path.to_string_lossy().to_string(),
        memory_mb: t.memory_mb,
        cpu_count: t.cpu_count,
        gpu_enabled: t.gpu_enabled,
        description: t.description,
        created_at: t.created_at.to_rfc3339(),
    }
}

fn pool_to_response(p: VMPool) -> PoolResponse {
    PoolResponse {
        id: p.id,
        name: p.name,
        template_id: p.template_id,
        desired_count: p.desired_count,
        warm_count: p.warm_count,
        created_at: p.created_at.to_rfc3339(),
    }
}

fn vm_to_response(v: VM) -> VMResponse {
    VMResponse {
        id: v.id,
        name: v.name,
        template_id: v.template_id,
        pool_id: v.pool_id,
        state: v.state.to_string(),
        ip_address: v.ip_address,
        memory_mb: v.memory_mb,
        cpu_count: v.cpu_count,
        gpu_enabled: v.gpu_enabled,
        created_at: v.created_at.to_rfc3339(),
        last_resumed_at: v.last_resumed_at.map(|t| t.to_rfc3339()),
    }
}
