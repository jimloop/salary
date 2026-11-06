use crate::app_commands::AppState;
use payroll_core::rules::{Rule,RuleStatus};
use payroll_storage::Collection;
use serde::{Deserialize,Serialize};
use std::{collections::{BTreeMap},fs};
use tauri::State;

#[derive(Serialize,Deserialize)]
struct Pack { schema:u32,rules:Vec<Rule> }
#[derive(Serialize)]
pub struct PackPreview { pub rules:Vec<Rule>,pub conflicts:Vec<String> }
fn all_rules(session:&payroll_storage::Session)->Result<Vec<Rule>,String>{
    let mut result=Vec::new();let mut page=1;
    loop{let batch=session.page::<Rule>(Collection::Rules,page,100)?;result.extend(batch.records);if page as u64*100>=batch.total{break;}page+=1;}
    Ok(result)
}
fn read_pack(path:&str)->Result<Pack,String>{
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    if bytes.len()>2_000_000{return Err("规则包超过 2 MB 上限".into());}
    let pack:Pack=serde_json::from_slice(&bytes).map_err(|_|"规则包格式无效")?;
    if pack.schema!=1{return Err("规则包版本不受支持".into());}
    let mut latest=BTreeMap::<String,Rule>::new();
    for rule in &pack.rules{if latest.get(&rule.code).is_none_or(|old|rule.version>old.version){latest.insert(rule.code.clone(),rule.clone());}}
    crate::payroll_rules::validate_configured(&latest.into_values().collect::<Vec<_>>())?;
    Ok(pack)
}

#[tauri::command]pub fn rules_export_pack(state:State<AppState>,path:String)->Result<usize,String>{
    state.with_session(|session|{
        let mut rules=all_rules(session)?;
        for rule in &mut rules{rule.source="共享规则包".into();rule.status=RuleStatus::PendingValidation;}
        let count=rules.len();
        fs::write(path,serde_json::to_vec_pretty(&Pack{schema:1,rules}).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        Ok(count)
    })
}
#[tauri::command]pub fn rules_preview_pack(state:State<AppState>,path:String)->Result<PackPreview,String>{
    state.with_session(|session|{
        let pack=read_pack(&path)?;
        let existing=all_rules(session)?;
        let conflicts=pack.rules.iter().filter(|rule|existing.iter().any(|old|old.code==rule.code)).map(|rule|rule.code.clone()).collect();
        Ok(PackPreview{rules:pack.rules,conflicts})
    })
}
#[tauri::command]pub fn rules_import_pack(state:State<AppState>,path:String)->Result<usize,String>{
    state.with_session_mut(|session|{
        let pack=read_pack(&path)?;
        let mut versions=BTreeMap::<String,u32>::new();
        for rule in all_rules(session)?{versions.entry(rule.code).and_modify(|v|*v=(*v).max(rule.version)).or_insert(rule.version);}
        let mut records=Vec::new();
        for mut rule in pack.rules{
            let next=versions.get(&rule.code).copied().unwrap_or(0).checked_add(1).ok_or("规则版本超出范围")?;
            versions.insert(rule.code.clone(),next);
            rule.version=next;rule.status=RuleStatus::PendingValidation;rule.source="共享规则包（待验证）".into();
            records.push((format!("{}-{}",rule.code,rule.version),serde_json::to_string(&rule).map_err(|e|e.to_string())?));
        }
        session.insert_rules_batch(&records)?;
        Ok(records.len())
    })
}
