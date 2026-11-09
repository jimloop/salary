use crate::{app_commands::{AppState,Employee,MonthlySnapshot},export_runs::selected_snapshots};
use payroll_core::Decimal;
use payroll_storage::Collection;
use serde::Serialize;
use std::collections::{BTreeMap,BTreeSet};
use tauri::State;

const LABELS:[&str;8]=["固定薪","绩效","调整项","应发工资","员工承担","本期预扣个税","实发工资","单位成本"];
fn values(v:&MonthlySnapshot)->[Option<Decimal>;8]{
    [Some(v.fixed_salary),Some(v.performance),Some(v.adjustment),Some(v.result.gross),Some(v.result.employee_deductions),v.result.withholding.as_ref().map(|w|w.current_withholding),v.result.net,Some(v.result.employer_cost)]
}
#[derive(Serialize)]
pub struct MetricDiff{label:String,before:Option<Decimal>,after:Option<Decimal>,delta:Option<Decimal>}
#[derive(Serialize)]
pub struct EmployeeDiff{employee_id:String,name:String,status:String,metrics:Vec<MetricDiff>,configuration_changes:Vec<ConfigurationDiff>}
#[derive(Serialize)]
pub struct ConfigurationDiff{project:String,field:String,before:Option<Decimal>,after:Option<Decimal>}
#[derive(Serialize)]
pub struct TotalDiff{label:String,before:Decimal,after:Decimal,delta:Option<Decimal>,missing_before:usize,missing_after:usize}
#[derive(Serialize)]
pub struct RunComparison{pub rows:Vec<EmployeeDiff>,pub totals:Vec<TotalDiff>}

fn compare(left:&[MonthlySnapshot],right:&[MonthlySnapshot],names:&BTreeMap<String,String>)->RunComparison{
    let projects:BTreeSet<String>=left.iter().chain(right).flat_map(|v|v.contributions.iter().flatten().map(|r|r.project.clone())).collect();
    let mut labels:Vec<String>=LABELS.iter().map(|v|v.to_string()).collect();
    for project in &projects{labels.push(format!("{project} · 员工承担"));labels.push(format!("{project} · 单位承担"));}
    let amounts=|v:&MonthlySnapshot|{let mut values=values(v).to_vec();for project in &projects{for employee in [true,false]{values.push(v.contributions.as_ref().map(|rows|rows.iter().find(|r|&r.project==project).map(|r|if employee{r.employee_amount}else{r.employer_amount}).unwrap_or(Decimal::ZERO)));}}values};
    let a:BTreeMap<_,_>=left.iter().map(|v|(v.employee_id.as_str(),v)).collect();
    let b:BTreeMap<_,_>=right.iter().map(|v|(v.employee_id.as_str(),v)).collect();
    let ids:BTreeSet<_>=a.keys().chain(b.keys()).copied().collect();
    let rows=ids.into_iter().map(|id|{
        let before=a.get(id).map(|v|amounts(v));let after=b.get(id).map(|v|amounts(v));
        let status=match (&before,&after){
            (None,_)=>"added",(_,None)=>"removed",
            (Some(x),Some(y)) if x!=y=>"changed",
            (Some(x),_) if x.iter().any(Option::is_none)=>"incomplete",
            _=>"same",
        };
        let metrics=labels.iter().enumerate().map(|(i,label)|{
            let x=before.as_ref().and_then(|v|v[i]);let y=after.as_ref().and_then(|v|v[i]);
            MetricDiff{label:label.to_string(),before:x,after:y,delta:x.zip(y).map(|(x,y)|y-x)}
        }).collect();
        let mut configuration_changes=Vec::new();
        for project in &projects { for (field,key) in [("缴费基数",0),("员工比例（%）",1),("单位比例（%）",2)] {
            let value=|snapshot:Option<&&MonthlySnapshot>|snapshot.and_then(|s|s.contributions.as_ref()).and_then(|rows|rows.iter().find(|r|&r.project==project)).map(|r|match key{0=>r.base.value,1=>r.employee_percent.value,_=>r.employer_percent.value});
            let before=value(a.get(id));let after=value(b.get(id));if before!=after{configuration_changes.push(ConfigurationDiff{project:project.clone(),field:field.into(),before,after});}
        }}
        EmployeeDiff{configuration_changes,employee_id:id.into(),name:names.get(id).cloned().unwrap_or_else(||"未命名员工".into()),status:status.into(),metrics}
    }).collect();
    let totals=labels.iter().enumerate().map(|(i,label)|{
        let sum=|versions:&[MonthlySnapshot]|{let mut total=Decimal::ZERO;let mut missing=0;for v in versions{match amounts(v)[i]{Some(value)=>total+=value,None=>missing+=1}}(total,missing)};
        let (before,missing_before)=sum(left);let (after,missing_after)=sum(right);
        TotalDiff{label:label.to_string(),before,after,delta:if missing_before+missing_after==0{Some(after-before)}else{None},missing_before,missing_after}
    }).collect();
    RunComparison{rows,totals}
}

#[tauri::command]
pub fn runs_compare(state:State<AppState>,left_id:String,right_id:String)->Result<RunComparison,String>{
    state.with_session(|session|{
        if left_id==right_id{return Err("请选择两个不同的试算版本".into());}
        let (left_run,left)=selected_snapshots(session,&left_id)?;
        let (right_run,right)=selected_snapshots(session,&right_id)?;
        if left_run.period!=right_run.period||left_run.payout_month!=right_run.payout_month{return Err("请选择同一所属期和计划发放月的版本".into());}
        let mut names=BTreeMap::new();
        for v in left.iter().chain(&right){if !names.contains_key(&v.employee_id){
            if let Some(employee)=session.get::<Employee>(Collection::Employees,&v.employee_id)?{names.insert(v.employee_id.clone(),employee.name);}
        }}
        Ok(compare(&left,&right,&names))
    })
}

#[tauri::command]
pub fn employee_versions_compare(state:State<AppState>,left_id:String,right_id:String)->Result<RunComparison,String>{
    state.with_session(|session|{
        let left:MonthlySnapshot=session.get(Collection::Calculations,&left_id)?.ok_or("原版本不存在")?;
        let right:MonthlySnapshot=session.get(Collection::Calculations,&right_id)?.ok_or("新版本不存在")?;
        if left.subject_id!=session.subject.id||right.subject_id!=session.subject.id||left.employee_id!=right.employee_id||left.period!=right.period||left.payout_month!=right.payout_month{return Err("请选择同一员工、同一期间的版本".into());}
        let names=crate::export_runs::employee_names(session,&[left.clone()])?;
        Ok(compare(&[left],&[right],&names))
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    fn snapshot(id:&str)->MonthlySnapshot{
        serde_json::from_value(serde_json::json!({"id":id,"subject_id":"s","employee_id":id,"period":"2026-10","payout_month":"2026-11","fixed_salary":"10000","performance":"0","adjustment":"0","result":{"gross":"10000","employee_deductions":"800","withholding":null,"net":"9200","employer_cost":"11200"},"input":{"employee_id":id,"period":"2026-10","payout_month":"2026-11","standard_days":"22","actual_days":"22","performance":"0","adjustment":"0","employee_deductions":"800","employer_contributions":"1200"}})).unwrap()
    }
    #[test]
    fn compares_each_employee_and_aggregates_salary_projects(){
        let left=vec![snapshot("E1"),snapshot("E2"),snapshot("E3")];
        let mut right=vec![snapshot("E1"),snapshot("E2"),snapshot("E4")];
        right[1].performance=Decimal::from(100);right[1].result.gross+=Decimal::from(100);
        let report=compare(&left,&right,&BTreeMap::from([("E2".into(),"林知远".into())]));
        assert_eq!(report.rows.iter().map(|r|r.status.as_str()).collect::<Vec<_>>(),["incomplete","changed","removed","added"]);
        assert_eq!(report.rows[1].name,"林知远");
        assert_eq!(report.rows[1].metrics[1].delta,Some(Decimal::from(100)));
        assert_eq!(report.totals[3].delta,Some(Decimal::from(100)));
        assert_eq!(report.totals[5].missing_before,3);
        assert_eq!(report.totals[5].delta,None);
        let mut complete=snapshot("E1");
        complete.result.withholding=Some(serde_json::from_value(serde_json::json!({"current_withholding":"0","cumulative_taxable_base":"0","rate":"0","quick_deduction":"0","cumulative_tax_due":"0"})).unwrap());
        assert_eq!(compare(&[complete.clone()],&[complete],&BTreeMap::new()).rows[0].status,"same");
    }
    #[test]
    fn unchanged_amounts_keep_same_status_while_configuration_changes_are_reported(){
        let mut before=snapshot("E1");before.result.withholding=Some(serde_json::from_value(serde_json::json!({"current_withholding":"0","cumulative_taxable_base":"0","rate":"0","quick_deduction":"0","cumulative_tax_due":"0"})).unwrap());
        let mut configuration=payroll_core::contributions::Configuration{id:"default".into(),employee_id:None,project:"住房公积金".into(),start:"2026-01".into(),end:None,enabled:true,base:Some(10000.into()),employee_percent:Some(Decimal::new(25,1)),employer_percent:Some(5.into())};
        before.contributions=Some(payroll_core::contributions::resolve(&[configuration.clone()],"E1","2026-10").unwrap());let mut after=before.clone();configuration.base=Some(5000.into());configuration.employee_percent=Some(5.into());configuration.employer_percent=Some(10.into());after.contributions=Some(payroll_core::contributions::resolve(&[configuration],"E1","2026-10").unwrap());
        let report=compare(&[before],&[after],&BTreeMap::new());assert_eq!(report.rows[0].status,"same");assert_eq!(report.rows[0].configuration_changes.len(),3);assert_eq!(report.rows[0].metrics.len(),10);assert_eq!(report.totals[8].delta,Some(Decimal::ZERO));
    }

}
