#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_commands;
mod contribution_config;
mod backup;
mod batch;
mod import_inputs;
mod import_view;
mod payroll_rules;
mod work_progress;
mod item_catalog;
mod calculation_view;
mod bonus_scheme;
mod rule_pack;
mod xlsx_export;
mod export_runs;
mod comparison;

use app_commands::AppState;
use payroll_storage::Store;
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = app.path().app_local_data_dir()?.join("workspace-v3");
            let store = Store::new(root).map_err(std::io::Error::other)?;
            app.manage(AppState { store, current: Mutex::new(None) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_commands::subjects_list, app_commands::subjects_create, app_commands::subjects_unlock,
            app_commands::subjects_lock, app_commands::subjects_delete,
            contribution_config::contribution_configuration_delete, contribution_config::contribution_configurations, contribution_config::contribution_configuration_save, contribution_config::contribution_preview, contribution_config::contribution_status, contribution_config::salary_effective,
            app_commands::employees_page, app_commands::employees_save,
            app_commands::salary_plans_page, app_commands::salary_plans_save,
            calculation_view::calculation_employees_page, calculation_view::calculation_employee_versions, app_commands::items_page, app_commands::items_save,
            work_progress::workspace_progress, payroll_rules::payroll_effective_rules, app_commands::rules_page, app_commands::rules_save,
            app_commands::payroll_calculate, app_commands::payroll_versions, export_runs::export_runs,
            import_view::imports_employee_data, import_view::imports_overview, import_view::imports_data_page, app_commands::imports_preview, app_commands::imports_validate, app_commands::imports_commit, app_commands::imports_page, app_commands::imports_set_withdrawn,
            app_commands::inference_analyze,
            bonus_scheme::bonus_schemes_list, bonus_scheme::bonus_schemes_save, import_inputs::bonus_inputs, app_commands::bonus_calculate, app_commands::bonus_versions,
            app_commands::versions_compare, comparison::runs_compare, comparison::employee_versions_compare, app_commands::audit_page, app_commands::audit_clear,
            app_commands::exports_payroll_csv,
            xlsx_export::exports_payroll_xlsx,
            backup::backup_export, backup::backup_restore,
            batch::payroll_calculate_batch,
            rule_pack::rules_export_pack, rule_pack::rules_preview_pack, rule_pack::rules_import_pack,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Salary Assistant");
}
