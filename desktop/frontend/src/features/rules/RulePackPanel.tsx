import {useState} from 'react';
import {open,save} from '@tauri-apps/plugin-dialog';
import {api} from '../../api';
import type {RulePackPreview} from '../../types';
export function RulePackPanel({onImported,compact=false}:{onImported:()=>void;compact?:boolean}){
  const [path,setPath]=useState('');const [preview,setPreview]=useState<RulePackPreview|null>(null);const [message,setMessage]=useState('');const [busy,setBusy]=useState(false);
  const exportPack=async()=>{setBusy(true);setMessage('');try{const target=await save({defaultPath:'工资规则包.salaryrules.json',filters:[{name:'工资规则包',extensions:['json']}]});if(!target)return;const count=await api.rulesExportPack(target);setMessage(`已导出 ${count} 条规则；规则包不包含员工与工资金额。`);}catch(e){setMessage(String(e));}finally{setBusy(false);}};
  const choose=async()=>{setBusy(true);setMessage('');try{const source=await open({multiple:false,filters:[{name:'工资规则包',extensions:['json']}]});if(typeof source!=='string')return;setPath(source);setPreview(await api.rulesPreviewPack(source));}catch(e){setMessage(String(e));}finally{setBusy(false);}};
  const importPack=async()=>{setBusy(true);setMessage('');try{const count=await api.rulesImportPack(path);setPreview(null);setPath('');setMessage(`已导入 ${count} 条独立待验证规则，请回算确认后启用。`);onImported();}catch(e){setMessage(String(e));}finally{setBusy(false);}};
  return <div className={compact?"rule-pack-compact":"panel"}><div className="panel-head">{!compact&&<div><h2>规则包分享</h2><p>导出只包含规则定义。导入先预览冲突，再创建当前主体的待验证版本。</p></div>}<div className="import-tools"><button disabled={busy} onClick={choose}>导入规则包</button><button disabled={busy} onClick={exportPack}>导出规则包</button></div></div>{preview&&<div className="pack-preview"><strong>待导入 {preview.rules.length} 条 · 与现有项目编码重合 {preview.conflicts.length} 条</strong>{preview.conflicts.length>0&&<p>冲突项目：{[...new Set(preview.conflicts)].join('、')}。导入会新增版本，不覆盖当前规则。</p>}{preview.rules.map((rule,i)=><div key={i}><code>{rule.code}</code> · {rule.label} · <code>{rule.expression}</code></div>)}<button className="primary" disabled={busy} onClick={importPack}>确认导入为待验证版本</button></div>}{message&&<div className="validation-ok" role="status">{message}</div>}</div>;
}
