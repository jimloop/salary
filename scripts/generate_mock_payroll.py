"""Create independent, deterministic Chinese payroll fixtures for two subjects.

These are fictional people and entities. Expected tax is calculated with Python Decimal,
separately from the Rust payroll engine. No employer data is imported.
"""
from __future__ import annotations

import csv
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "mock-payroll"
D = Decimal


def cent(value: Decimal) -> Decimal:
    return value.quantize(D("0.01"), rounding=ROUND_HALF_UP)


def tax(base: Decimal) -> Decimal:
    if base <= 0:
        return D(0)
    for ceiling, rate, quick in [
        (D(36000), D("0.03"), D(0)),
        (D(144000), D("0.10"), D(2520)),
        (D(300000), D("0.20"), D(16920)),
        (D(420000), D("0.25"), D(31920)),
        (D(660000), D("0.30"), D(52920)),
        (D(960000), D("0.35"), D(85920)),
        (D("1e30"), D("0.45"), D(181920)),
    ]:
        if base <= ceiling:
            return cent(base * rate - quick)
    raise AssertionError("unreachable")


def write(name: str, rows: list[dict]) -> None:
    path = ROOT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8-sig", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


subjects = [
    ("E-A", "云杉科技", [
        ("A-1001", "林知远", D(10000), D(90000), D(45000), D(9000), D(800), D(0), D(1080)),
        ("A-1002", "陈亦凡", D(12000), D(108000), D(45000), D(9000), D(1200), D(1000), D(2880)),
    ]),
    ("E-B", "海棠咨询", [
        ("B-2001", "周一宁", D(8000), D(72000), D(45000), D(7200), D(600), D(0), D(594)),
        ("B-2002", "王若安", D(18000), D(162000), D(45000), D(13500), D(1500), D(2000), D(7830)),
    ]),
]

for subject_id, subject_name, people in subjects:
    employees = []
    attendance = []
    performance_rows = []
    adjustment_rows = []
    baseline_rows = []
    social_rows = []
    expected = []
    for employee_id, name, base_salary, prior_income, prior_basic, prior_special, employee_social, special_additional, prior_tax in people:
        employees.append({"主体ID": subject_id, "工号": employee_id, "姓名": name, "固定月薪": base_salary, "部门": "产品部" if employee_id.endswith("1") else "研发部"})
        cumulative_income = prior_income
        cumulative_basic = prior_basic
        cumulative_special = prior_special
        cumulative_additional = D(0)
        withheld = prior_tax
        for period, payout, actual_days, performance, adjustment in [
            ("2026-10", "2026-11", D(21), D(1000), D(0)),
            ("2026-11", "2026-12", D(20), D(1500), D(-200)),
        ]:
            standard_days = D(22)
            fixed = cent(base_salary / standard_days * actual_days)
            gross = cent(fixed + performance + adjustment)
            baseline_rows.append({"主体ID":subject_id,"工号":employee_id,"计划发放月":payout,
                "此前累计收入":cent(cumulative_income),"此前累计基本减除费用":cent(cumulative_basic),
                "此前累计专项扣除":cent(cumulative_special),"此前累计专项附加扣除":cent(cumulative_additional),
                "此前累计免税收入":D(0),"此前累计其他扣除":D(0),"此前累计减免税额":D(0),
                "此前实际已预扣个税":cent(withheld)})
            social_rows.append({"主体ID":subject_id,"所属期":period,"工号":employee_id,"员工承担":employee_social,"单位承担":cent(employee_social*D("1.5"))})
            attendance.append({"主体ID":subject_id,"所属期":period,"计划发放月":payout,"工号":employee_id,
                "标准工作日":standard_days,"实出勤日":actual_days,"绩效奖金":performance,"调整项":adjustment,
                "员工社保公积金":employee_social,"专项附加扣除":special_additional})
            performance_rows.append({"主体ID":subject_id,"所属期":period,"工号":employee_id,"绩效奖金":performance})
            adjustment_rows.append({"主体ID":subject_id,"所属期":period,"工号":employee_id,"调整项":adjustment})
            cumulative_income += gross
            cumulative_basic += D(5000)
            cumulative_special += employee_social
            cumulative_additional += special_additional
            cumulative_base = cumulative_income - cumulative_basic - cumulative_special - cumulative_additional
            cumulative_due = tax(cumulative_base)
            current_tax = max(D(0), cent(cumulative_due - withheld))
            net = cent(gross - employee_social - current_tax)
            expected.append({"主体ID":subject_id,"主体名称":subject_name,"所属期":period,"计划发放月":payout,
                "工号":employee_id,"姓名":name,"计薪固定薪":fixed,"绩效奖金":performance,"调整项":adjustment,
                "应发工资":gross,"员工社保公积金":employee_social,"累计应纳税所得额":cent(cumulative_base),
                "累计应预扣税额":cumulative_due,"本期预扣个税":current_tax,"实发工资":net})
            withheld += current_tax
    write(f"{subject_id}/employees.csv",employees)
    write(f"{subject_id}/attendance.csv",attendance)
    write(f"{subject_id}/performance.csv",performance_rows)
    write(f"{subject_id}/adjustments.csv",adjustment_rows)
    write(f"{subject_id}/tax_baseline.csv",baseline_rows)
    write(f"{subject_id}/social.csv",social_rows)
    write(f"{subject_id}/expected_payroll.csv",expected)

print(f"Created independent mock payroll fixtures under {ROOT}")
