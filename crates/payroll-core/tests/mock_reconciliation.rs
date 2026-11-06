use payroll_core::{monthly::{calculate_pay, CurrentTaxInput, TaxBaseline}, Decimal};
use rust_decimal::RoundingStrategy;
use std::{collections::BTreeMap, str::FromStr};

fn rows(csv: &str) -> Vec<BTreeMap<String, String>> {
    let mut lines = csv.trim_start_matches('\u{feff}').lines();
    let headers: Vec<&str> = lines.next().unwrap().trim_end_matches('\r').split(',').collect();
    lines.map(|line| headers.iter().zip(line.trim_end_matches('\r').split(',')).map(|(h,v)|((*h).into(),v.into())).collect()).collect()
}
fn dec(row:&BTreeMap<String,String>,key:&str)->Decimal {Decimal::from_str(&row[key]).unwrap()}

#[test]
fn two_subjects_two_months_match_independent_expected_results() {
    for (employees_csv,attendance_csv,baseline_csv,expected_csv) in [
        (include_str!("../../../tests/fixtures/mock-payroll/E-A/employees.csv"),include_str!("../../../tests/fixtures/mock-payroll/E-A/attendance.csv"),include_str!("../../../tests/fixtures/mock-payroll/E-A/tax_baseline.csv"),include_str!("../../../tests/fixtures/mock-payroll/E-A/expected_payroll.csv")),
        (include_str!("../../../tests/fixtures/mock-payroll/E-B/employees.csv"),include_str!("../../../tests/fixtures/mock-payroll/E-B/attendance.csv"),include_str!("../../../tests/fixtures/mock-payroll/E-B/tax_baseline.csv"),include_str!("../../../tests/fixtures/mock-payroll/E-B/expected_payroll.csv")),
    ] {
        let employees=rows(employees_csv);let attendance=rows(attendance_csv);let baselines=rows(baseline_csv);let expected=rows(expected_csv);
        assert_eq!(expected.len(),4);
        for target in expected {
            let id=&target["工号"];let payout=&target["计划发放月"];
            let employee=employees.iter().find(|r|&r["工号"]==id).unwrap();
            let input=attendance.iter().find(|r|&r["工号"]==id&&&r["计划发放月"]==payout).unwrap();
            let history=baselines.iter().find(|r|&r["工号"]==id&&&r["计划发放月"]==payout).unwrap();
            let fixed=(dec(employee,"固定月薪")/dec(input,"标准工作日")*dec(input,"实出勤日")).round_dp_with_strategy(2,RoundingStrategy::MidpointAwayFromZero);
            let gross=fixed+dec(input,"绩效奖金")+dec(input,"调整项");
            let baseline=TaxBaseline{cumulative_income:dec(history,"此前累计收入"),cumulative_exempt_income:Decimal::ZERO,cumulative_basic_deduction:dec(history,"此前累计基本减除费用"),cumulative_special_deduction:dec(history,"此前累计专项扣除"),cumulative_additional_deduction:dec(history,"此前累计专项附加扣除"),cumulative_other_deduction:Decimal::ZERO,cumulative_tax_relief:Decimal::ZERO,cumulative_withheld_tax:dec(history,"此前实际已预扣个税")};
            let current=CurrentTaxInput{taxable_income:gross,exempt_income:Decimal::ZERO,basic_deduction:Decimal::from(5000),special_deduction:dec(input,"员工社保公积金"),additional_deduction:dec(input,"专项附加扣除"),other_deduction:Decimal::ZERO,tax_relief:Decimal::ZERO};
            let result=calculate_pay(gross,current.special_deduction,Decimal::ZERO,Some((&baseline,&current))).unwrap();
            assert_eq!(fixed,dec(&target,"计薪固定薪"),"{id} {payout} fixed");
            assert_eq!(result.gross,dec(&target,"应发工资"),"{id} {payout} gross");
            let tax=result.withholding.unwrap();
            assert_eq!(tax.cumulative_taxable_base,dec(&target,"累计应纳税所得额"),"{id} {payout} tax base");
            assert_eq!(tax.current_withholding,dec(&target,"本期预扣个税"),"{id} {payout} tax");
            assert_eq!(result.net,Some(dec(&target,"实发工资")),"{id} {payout} net");
        }
    }
}
