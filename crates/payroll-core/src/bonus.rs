use serde::{Deserialize, Serialize};
use rust_decimal::RoundingStrategy;
use crate::{monthly::{progressive_tax, TaxBaseline}, Decimal};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BonusPlan {
    pub target: Decimal,
    pub performance_factor: Decimal,
    pub eligible_days: u32,
    pub plan_days: u32,
    pub minimum: Option<Decimal>,
    pub maximum: Option<Decimal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BonusAmount { pub before_limits: Decimal, pub awarded: Decimal }

pub fn award(plan: &BonusPlan) -> Result<BonusAmount, String> {
    if plan.target.is_sign_negative() || plan.performance_factor.is_sign_negative() || plan.plan_days == 0 || plan.eligible_days > plan.plan_days { return Err("年终奖资格或金额参数无效".into()); }
    if let (Some(min), Some(max)) = (plan.minimum, plan.maximum) { if min > max { return Err("年终奖保底高于封顶".into()); } }
    let before_limits = (plan.target * plan.performance_factor * Decimal::from(plan.eligible_days) / Decimal::from(plan.plan_days)).round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    let awarded = before_limits.max(plan.minimum.unwrap_or(Decimal::ZERO)).min(plan.maximum.unwrap_or(Decimal::MAX));
    Ok(BonusAmount { before_limits, awarded })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BonusTaxComparison { pub separate: Decimal, pub combined_increment: Decimal }

/// Separate annual-bonus brackets under 财政部 税务总局公告 2023 年第 30 号 (valid through 2027-12-31).
pub fn separate_bonus_tax(amount: Decimal) -> Result<Decimal, String> {
    if amount.is_sign_negative() { return Err("奖金金额不能为负".into()); }
    let monthly_equivalent = amount / Decimal::from(12);
    let (rate_pct, quick) = if monthly_equivalent <= Decimal::from(3000) { (3, 0) }
        else if monthly_equivalent <= Decimal::from(12000) { (10, 210) }
        else if monthly_equivalent <= Decimal::from(25000) { (20, 1410) }
        else if monthly_equivalent <= Decimal::from(35000) { (25, 2660) }
        else if monthly_equivalent <= Decimal::from(55000) { (30, 4410) }
        else if monthly_equivalent <= Decimal::from(80000) { (35, 7160) }
        else { (45, 15160) };
    Ok((amount * Decimal::from(rate_pct) / Decimal::from(100) - Decimal::from(quick)).round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero))
}

/// Comparison only. User must confirm treatment, and actual cross-employer annual data cannot be inferred.
pub fn compare_tax(amount: Decimal, baseline: &TaxBaseline) -> Result<BonusTaxComparison, String> {
    let separate = separate_bonus_tax(amount)?;
    let before = baseline.cumulative_income - baseline.cumulative_exempt_income - baseline.cumulative_basic_deduction
        - baseline.cumulative_special_deduction - baseline.cumulative_additional_deduction - baseline.cumulative_other_deduction;
    let combined_increment = (progressive_tax(before + amount).2 - progressive_tax(before).2).max(Decimal::ZERO);
    Ok(BonusTaxComparison { separate, combined_increment })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn annual_bonus_bracket_boundary() {
        assert_eq!(separate_bonus_tax(Decimal::from(36000)).unwrap(), Decimal::from(1080));
        assert_eq!(separate_bonus_tax(Decimal::from(36001)).unwrap(), Decimal::new(339010, 2));
    }
    #[test] fn award_applies_proration_and_cap() {
        let plan = BonusPlan { target: Decimal::from(10000), performance_factor: Decimal::from(2), eligible_days: 180, plan_days: 360, minimum: None, maximum: Some(Decimal::from(9000)) };
        assert_eq!(award(&plan).unwrap().awarded, Decimal::from(9000));
    }
}
