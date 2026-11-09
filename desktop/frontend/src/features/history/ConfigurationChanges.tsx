import type { RunComparison } from '../../types';
export function ConfigurationChanges({rows}:{rows:RunComparison['rows']}) {
 const changed=rows.filter(row=>row.configuration_changes?.length);
 if(!changed.length)return null;
 return <section className="panel"><div className="panel-head"><h3>基数 / 比例变化</h3></div><p className="prototype-helper">金额一致时仍单独列出配置变化；配置变化不改变金额对比状态。</p><div className="table-wrap"><table><thead><tr><th>员工</th><th>项目</th><th>配置字段</th><th>原版本</th><th>新版本</th></tr></thead><tbody>{changed.flatMap(row=>row.configuration_changes!.map(change=><tr className="comparison-row comparison-changed" key={`${row.employee_id}-${change.project}-${change.field}`}><td>{row.name} · {row.employee_id}</td><td>{change.project}</td><td>{change.field}</td><td>{change.before??'旧版未保存 / 未启用'}</td><td>{change.after??'旧版未保存 / 未启用'}</td></tr>))}</tbody></table></div></section>;
}
