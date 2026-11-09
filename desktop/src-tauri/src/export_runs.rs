use crate::app_commands::{AppState,Employee,MonthlySnapshot};
use payroll_storage::{Collection,Session};
use serde::{Deserialize,Serialize};
use std::{collections::{BTreeMap,BTreeSet},time::{SystemTime,UNIX_EPOCH}};
use tauri::State;
use uuid::Uuid;

pub fn employee_names(session:&Session,versions:&[MonthlySnapshot])->Result<BTreeMap<String,String>,String>{
    let mut names=BTreeMap::new();
    for version in versions{
        if !names.contains_key(&version.employee_id){
            let employee:Employee=session.get(Collection::Employees,&version.employee_id)?.ok_or("导出员工档案不存在")?;
            names.insert(version.employee_id.clone(),employee.name);
        }
    }
    Ok(names)
}

#[derive(Clone,Serialize,Deserialize)]
pub struct CalculationRun {
    #[serde(default)] pub needs_recalculation:bool,
    pub id:String,
    pub subject_id:String,
    pub period:String,
    pub payout_month:String,
    pub version:u32,
    pub kind:String,
    pub created_at:u64,
    pub version_ids:Vec<String>,
}

impl CalculationRun {
    pub fn new(session:&Session,period:&str,payout_month:&str,kind:&str,version_ids:Vec<String>)->Result<Self,String>{
        if version_ids.is_empty(){return Err("本次试算没有员工结果".into());}
        Ok(Self{needs_recalculation:false,
            id:Uuid::new_v4().to_string(),subject_id:session.subject.id.clone(),period:period.into(),payout_month:payout_month.into(),
            version:session.next_calculation_version(period,payout_month)?,kind:kind.into(),
            created_at:SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs(),version_ids,
        })
    }
}

#[tauri::command]
pub fn export_runs(state:State<AppState>,period:String,payout_month:String)->Result<Vec<CalculationRun>,String>{
    state.with_session(|session|{
        let mut found=Vec::new();let mut page=1;
        loop{
            let batch=session.page::<CalculationRun>(Collection::CalculationRuns,page,100)?;
            found.extend(batch.records.into_iter().filter(|run|run.subject_id==session.subject.id&&run.period==period&&run.payout_month==payout_month));
            if page as u64*100>=batch.total{break;}page+=1;
        }
        for run in &mut found {let mut snapshots=selected_snapshots(session,&run.id)?.1;crate::work_progress::mark_recalculation(session,&mut snapshots)?;run.needs_recalculation=snapshots.iter().any(|v|v.needs_recalculation);}
        Ok(found)
    })
}

pub fn selected_snapshots(session:&Session,run_id:&str)->Result<(CalculationRun,Vec<MonthlySnapshot>),String>{
    let run:CalculationRun=session.get(Collection::CalculationRuns,run_id)?.ok_or("试算版本不存在")?;
    if run.subject_id!=session.subject.id||run.version_ids.is_empty(){return Err("试算版本不属于当前主体或没有员工结果".into());}
    let mut employees=BTreeSet::new();let mut snapshots=Vec::with_capacity(run.version_ids.len());
    for id in &run.version_ids{
        let snapshot:MonthlySnapshot=session.get(Collection::Calculations,id)?.ok_or_else(||format!("计算结果 {id} 不存在"))?;
        if snapshot.subject_id!=run.subject_id||snapshot.period!=run.period||snapshot.payout_month!=run.payout_month||!employees.insert(snapshot.employee_id.clone()){
            return Err("试算版本中的员工结果不一致".into());
        }
        snapshots.push(snapshot);
    }
    Ok((run,snapshots))
}

#[cfg(test)]
mod tests{
    use super::*;
    use payroll_storage::Store;

    fn snapshot(subject:&str,employee:&str,id:&str)->MonthlySnapshot{
        serde_json::from_value(serde_json::json!({
            "id":id,"subject_id":subject,"employee_id":employee,"period":"2026-10","payout_month":"2026-11",
            "fixed_salary":"10000","performance":"0","adjustment":"0",
            "result":{"gross":"10000","employee_deductions":"800","withholding":null,"net":null,"employer_cost":"11200"},
            "input":{"employee_id":employee,"period":"2026-10","payout_month":"2026-11","standard_days":"22","actual_days":"22","performance":"0","adjustment":"0","employee_deductions":"800","employer_contributions":"1200"}
        })).unwrap()
    }

    #[test]
    fn run_selection_keeps_each_batch_together_after_reopen(){
        let root=std::env::temp_dir().join(format!("salary-run-test-{}",Uuid::new_v4()));
        let store=Store::new(&root).unwrap();
        let subject=store.create_subject("版本主体","1234","上海","test-password").unwrap();
        let mut session=store.unlock(&subject.id,"test-password").unwrap();
        let mut run_ids=Vec::new();
        for version in 1..=2{
            let snapshots=[snapshot(&subject.id,"E1",&format!("E1-v{version}")),snapshot(&subject.id,"E2",&format!("E2-v{version}"))];
            let ids=snapshots.iter().map(|value|value.id.clone()).collect();
            let run=CalculationRun::new(&session,"2026-10","2026-11","batch",ids).unwrap();
            assert_eq!(run.version,version);
            let records=snapshots.iter().map(|value|(value.id.clone(),serde_json::to_string(value).unwrap())).collect::<Vec<_>>();
            session.insert_calculation_run(&run.id,&run,&records,"calculate_batch","2 records").unwrap();
            run_ids.push(run.id);
        }
        drop(session);
        let mut reopened=store.unlock(&subject.id,"test-password").unwrap();
        let (first,first_rows)=selected_snapshots(&reopened,&run_ids[0]).unwrap();
        let (second,second_rows)=selected_snapshots(&reopened,&run_ids[1]).unwrap();
        assert_eq!((first.version,second.version),(1,2));
        assert_eq!(first_rows.iter().map(|value|value.id.as_str()).collect::<Vec<_>>(),vec!["E1-v1","E2-v1"]);
        assert_eq!(second_rows.iter().map(|value|value.id.as_str()).collect::<Vec<_>>(),vec!["E1-v2","E2-v2"]);
        assert!(selected_snapshots(&reopened,"missing").is_err());
        let invalid=CalculationRun::new(&reopened,"2026-10","2026-11","batch",vec!["E1-v1".into()]).unwrap();
        let duplicate=vec![("E1-v1".into(),serde_json::to_string(&first_rows[0]).unwrap())];
        assert!(reopened.insert_calculation_run(&invalid.id,&invalid,&duplicate,"calculate_batch","1 records").is_err());
        assert!(reopened.get::<CalculationRun>(Collection::CalculationRuns,&invalid.id).unwrap().is_none());
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
}
