use serde::{Deserialize, Serialize};
use rust_decimal::RoundingStrategy;
use crate::Decimal;

fn money(value: Decimal) -> Decimal { value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero) }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxBaseline {
    pub cumulative_income: Decimal,
    pub cumulative_exempt_income: Decimal,
    pub cumulative_basic_deduction: Decimal,
    pub cumulative_special_deduction: Decimal,
    pub cumulative_additional_deduction: Decimal,
    pub cumulative_other_deduction: Decimal,
    pub cumulative_tax_relief: Decimal,
    pub cumulative_withheld_tax: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrentTaxInput {
    pub taxable_income: Decimal,
    pub exempt_income: Decimal,
    pub basic_deduction: Decimal,
    pub special_deduction: Decimal,
    pub additional_deduction: Decimal,
    pub other_deduction: Decimal,
    pub tax_relief: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxResult {
    pub cumulative_taxable_base: Decimal,
    pub rate: Decimal,
    pub quick_deduction: Decimal,
    pub cumulative_tax_due: Decimal,
    pub current_withholding: Decimal,
}

pub fn progressive_tax(base: Decimal) -> (Decimal, Decimal, Decimal) {
    let zero = Decimal::ZERO;
    if base <= zero { return (zero, zero, zero); }
    let brackets = [
        (Decimal::from(36000), 3, 0),
        (Decimal::from(144000), 10, 2520),
        (Decimal::from(300000), 20, 16920),
        (Decimal::from(420000), 25, 31920),
        (Decimal::from(660000), 30, 52920),
        (Decimal::from(960000), 35, 85920),
        (Decimal::MAX, 45, 181920),
    ];
    for (ceiling, pct, quick) in brackets {
        if base <= ceiling {
            let rate = Decimal::from(pct) / Decimal::from(100);
            let quick_deduction = Decimal::from(quick);
            return (rate, quick_deduction, money(base * rate - quick_deduction));
        }
    }
    unreachable!()
}

/// Baseline is mandatory. Missing history must be reported by the caller, never silently zeroed.
pub fn cumulative_withholding(history: &TaxBaseline, current: &CurrentTaxInput) -> Result<TaxResult, String> {
    let all = [history.cumulative_income, history.cumulative_exempt_income, history.cumulative_basic_deduction,
        history.cumulative_special_deduction, history.cumulative_additional_deduction, history.cumulative_other_deduction,
        history.cumulative_tax_relief, history.cumulative_withheld_tax, current.exempt_income, current.basic_deduction,
        current.special_deduction, current.additional_deduction, current.other_deduction, current.tax_relief];
    if all.iter().any(|value| value.is_sign_negative()) { return Err("累计税务基线或扣除项不能为负".into()); }
    let base = history.cumulative_income + current.taxable_income
        - history.cumulative_exempt_income - current.exempt_income
        - history.cumulative_basic_deduction - current.basic_deduction
        - history.cumulative_special_deduction - current.special_deduction
        - history.cumulative_additional_deduction - current.additional_deduction
        - history.cumulative_other_deduction - current.other_deduction;
    let (rate, quick_deduction, gross_due) = progressive_tax(base);
    let due = (gross_due - history.cumulative_tax_relief - current.tax_relief).max(Decimal::ZERO);
    let current_withholding = (due - history.cumulative_withheld_tax).max(Decimal::ZERO);
    Ok(TaxResult { cumulative_taxable_base: base.max(Decimal::ZERO), rate, quick_deduction, cumulative_tax_due: due, current_withholding: money(current_withholding) })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PayResult { pub gross: Decimal, pub employee_deductions: Decimal, pub withholding: Option<TaxResult>, pub net: Option<Decimal>, pub employer_cost: Decimal }

pub fn calculate_pay(gross: Decimal, employee_deductions: Decimal, employer_contributions: Decimal, tax: Option<(&TaxBaseline, &CurrentTaxInput)>) -> Result<PayResult, String> {
    let withholding = tax.map(|(baseline, current)| cumulative_withholding(baseline, current)).transpose()?;
    let net = withholding.as_ref().map(|result| money(gross - employee_deductions - result.current_withholding));
    Ok(PayResult { gross: money(gross), employee_deductions: money(employee_deductions), withholding, net, employer_cost: money(gross + employer_contributions) })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn baseline() -> TaxBaseline { TaxBaseline { cumulative_income: Decimal::from(10000), cumulative_exempt_income: Decimal::ZERO, cumulative_basic_deduction: Decimal::from(5000), cumulative_special_deduction: Decimal::from(1000), cumulative_additional_deduction: Decimal::ZERO, cumulative_other_deduction: Decimal::ZERO, cumulative_tax_relief: Decimal::ZERO, cumulative_withheld_tax: Decimal::from(120) } }
    fn current() -> CurrentTaxInput { CurrentTaxInput { taxable_income: Decimal::from(10000), exempt_income: Decimal::ZERO, basic_deduction: Decimal::from(5000), special_deduction: Decimal::from(1000), additional_deduction: Decimal::ZERO, other_deduction: Decimal::ZERO, tax_relief: Decimal::ZERO } }
    #[test] fn cumulative_uses_actual_withheld_history() {
        let result = cumulative_withholding(&baseline(), &current()).unwrap();
        assert_eq!(result.cumulative_taxable_base, Decimal::from(8000));
        assert_eq!(result.current_withholding, Decimal::from(120));
        assert_eq!(calculate_pay(Decimal::from(10000), Decimal::from(1000), Decimal::from(2000), Some((&baseline(), &current()))).unwrap().net, Some(Decimal::from(8880)));
    }
    #[test] fn no_baseline_does_not_invent_zero_tax() { assert!(calculate_pay(Decimal::from(10000), Decimal::ZERO, Decimal::ZERO, None).unwrap().net.is_none()); }
    #[test] fn brackets_are_continuous() { assert_eq!(progressive_tax(Decimal::from(36000)).2, progressive_tax(Decimal::from(36000)).2); assert_eq!(progressive_tax(Decimal::from(144000)).2, Decimal::from(11880)); }
}
