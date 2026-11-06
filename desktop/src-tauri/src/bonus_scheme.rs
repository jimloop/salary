use crate::app_commands::{amount,AppState};
use payroll_storage::{Collection,Session};
use serde::{Deserialize,Serialize};
use tauri::State;
#[derive(Clone,Serialize,Deserialize)]
pub struct BonusScheme{pub id:String,pub name:String,pub year:String,pub budget:String,pub plan_days:u32,pub minimum:Option<String>,pub maximum:Option<String>}
pub fn save(session:&mut Session,mut scheme:BonusScheme)->Result<BonusScheme,String>{
 if scheme.name.trim().is_empty()||scheme.year.len()!=4||scheme.year.parse::<u32>().is_err()||scheme.plan_days==0{return Err("方案名称、四位奖金年度及正整数周期天数必填".into());}
 if amount(&scheme.budget,"预算池")?.is_sign_negative(){return Err("预算池不能为负".into());}
 let min=scheme.minimum.as_deref().map(|value|amount(value,"保底金额")).transpose()?;let max=scheme.maximum.as_deref().map(|value|amount(value,"封顶金额")).transpose()?;
 if min.is_some_and(|v|v.is_sign_negative())||max.is_some_and(|v|v.is_sign_negative())||min.zip(max).is_some_and(|(a,b)|a>b){return Err("封顶保底必须非负，且保底不高于封顶".into());}
 scheme.id=uuid::Uuid::new_v4().to_string();session.insert_immutable(Collection::BonusSchemes,&scheme.id,&scheme)?;Ok(scheme)
}
#[tauri::command]pub fn bonus_schemes_save(state:State<AppState>,scheme:BonusScheme)->Result<BonusScheme,String>{state.with_session_mut(|session|save(session,scheme))}
#[tauri::command]pub fn bonus_schemes_list(state:State<AppState>,year:String)->Result<Vec<BonusScheme>,String>{state.with_session(|session|{let mut result=Vec::new();let mut page=1;loop{let batch=session.page::<BonusScheme>(Collection::BonusSchemes,page,100)?;result.extend(batch.records.into_iter().filter(|scheme|scheme.year==year));if page as u64*100>=batch.total{break}page+=1;}Ok(result)})}
#[cfg(test)]mod tests{use super::*;#[test]fn schemes_are_immutable_and_budget_is_validated(){let root=std::env::temp_dir().join(format!("bonus-scheme-{}",uuid::Uuid::new_v4()));let store=payroll_storage::Store::new(root.clone()).unwrap();let subject=store.create_subject("奖金主体","1111","上海","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();let original=BonusScheme{id:String::new(),name:"年度方案".into(),year:"2026".into(),budget:"10000".into(),plan_days:365,minimum:None,maximum:None};let a=save(&mut session,original.clone()).unwrap();let mut changed=a.clone();changed.budget="20000".into();let b=save(&mut session,changed).unwrap();assert_ne!(a.id,b.id);assert_eq!(session.get::<BonusScheme>(Collection::BonusSchemes,&a.id).unwrap().unwrap().budget,"10000");let mut invalid=original;invalid.budget="-1".into();assert!(save(&mut session,invalid).is_err());drop(session);drop(store);let _=std::fs::remove_dir_all(root);}}
