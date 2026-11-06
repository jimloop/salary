import { ComboSelect } from './ComboSelect';
import type { Page } from '../types';
export function Pagination<T>({data,onPage,onSize}:{data:Page<T>;onPage:(page:number)=>void;onSize:(size:number)=>void}){
 const last=Math.max(1,Math.ceil(data.total/data.page_size));const current=Math.min(Math.max(1,data.page),last);
 const numbers=[...new Set([1,current-1,current,current+1,last].filter(page=>page>0&&page<=last))].sort((a,b)=>a-b);
 return <nav className="pagination" aria-label="列表分页"><span className="pagination-total">共 {data.total} 条</span><label>每页<ComboSelect aria-label="每页条数" value={data.page_size} onChange={event=>onSize(Number(event.target.value))}>{[20,50,100].map(size=><option key={size} value={size}>{size}</option>)}</ComboSelect>条</label><div className="pagination-pages"><button aria-label="上一页" disabled={current===1} onClick={()=>onPage(current-1)}>‹</button>{numbers.map((page,index)=><span className="pagination-page-slot" key={page}>{index>0&&page>numbers[index-1]+1&&<span className="pagination-ellipsis">…</span>}<button className={page===current?'active':''} aria-label={`第 ${page} 页`} aria-current={page===current?'page':undefined} onClick={()=>onPage(page)}>{page}</button></span>)}<button aria-label="下一页" disabled={current===last} onClick={()=>onPage(current+1)}>›</button></div><span>第 {current} / {last} 页</span></nav>;
}
