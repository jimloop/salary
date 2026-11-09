import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

type Props = {
  type: 'date' | 'month';
  value: string;
  onChange: (event: { target: { value: string } }) => void;
  disabled?: boolean;
};
const daysInMonth = (year: number, month: number) => month === 2 ? (year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28) : [4, 6, 9, 11].includes(month) ? 30 : 31;
function normalize(text: string, type: Props['type']): string | null {
  if (!text.trim()) return '';
  const parts = text.trim().match(/^(\d{4})[-/.年](\d{1,2})(?:[-/.月](\d{1,2})日?)?月?$/);
  if (!parts || (type === 'date') !== !!parts[3]) return null;
  const year = Number(parts[1]), month = Number(parts[2]), day = Number(parts[3]);
  if (year < 1 || month < 1 || month > 12) return null;
  if (type === 'date' && (day < 1 || day > daysInMonth(year, month))) return null;
  return `${parts[1]}-${String(month).padStart(2, '0')}${type === 'date' ? `-${String(day).padStart(2, '0')}` : ''}`;
}
export function DateField({ type, value, onChange, disabled }: Props) {
  const [draft, setDraft] = useState(value);
  const [invalid, setInvalid] = useState(false);
  const [open, setOpen] = useState(false);
  const [view, setView] = useState({year: new Date().getFullYear(), month: new Date().getMonth() + 1});
  const [position, setPosition] = useState({left: 0, top: 0});
  const anchor = useRef<HTMLSpanElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  useEffect(() => { setDraft(value); setInvalid(false); }, [value]);
  useEffect(() => { if (disabled) setOpen(false); }, [disabled]);
  useEffect(() => {
    if (!open) return;
    const closeOutside = (event: MouseEvent) => { if (!anchor.current?.contains(event.target as Node) && !panel.current?.contains(event.target as Node)) setOpen(false); };
    const closeOnMove = () => setOpen(false);
    document.addEventListener('mousedown', closeOutside, true);
    window.addEventListener('resize', closeOnMove);
    const closeOnScroll = (event: Event) => { if (!panel.current?.contains(event.target as Node)) setOpen(false); };
    document.addEventListener('scroll', closeOnScroll, true);
    panel.current?.querySelector<HTMLButtonElement>('[aria-pressed="true"], .calendar-choice')?.focus();
    return () => { document.removeEventListener('mousedown', closeOutside, true); window.removeEventListener('resize', closeOnMove); document.removeEventListener('scroll', closeOnScroll, true); };
  }, [open]);
  const commit = (text: string) => {
    const normalized = normalize(text, type);
    if (normalized === null) { setInvalid(true); return; }
    setDraft(normalized); setInvalid(false);
    if (normalized !== value) onChange({ target: { value: normalized } });
  };
  const close = () => { setOpen(false); toggle.current?.focus(); };
  const choose = (text: string) => { commit(text); close(); };
  const show = () => {
    if (open) { close(); return; }
    const parsed = normalize(draft, type);
    const now = new Date();
    setView({year: parsed ? Number(parsed.slice(0, 4)) : now.getFullYear(), month: parsed ? Number(parsed.slice(5, 7)) : now.getMonth() + 1});
    const rect = anchor.current!.getBoundingClientRect();
    const height = type === 'month' ? 270 : 410;
    setPosition({left: Math.max(8, Math.min(rect.left, window.innerWidth - 288)), top: rect.bottom + height + 8 <= window.innerHeight ? rect.bottom + 6 : Math.max(8, rect.top - height - 6)});
    setOpen(true);
  };
  const prefix = `${String(view.year).padStart(4, '0')}-${String(view.month).padStart(2, '0')}`;
  const firstDay = new Date(0); firstDay.setFullYear(view.year, view.month - 1, 1); firstDay.setHours(0, 0, 0, 0);
  const offset = (firstDay.getDay() + 6) % 7;
  return <span ref={anchor} className={`date-field${invalid ? ' invalid' : ''}`}>
    <input type="text" inputMode="numeric" value={draft} disabled={disabled} aria-invalid={invalid}
      aria-label={type === 'month' ? '输入月份' : '输入日期'} placeholder={type === 'month' ? 'YYYY-MM' : 'YYYY-MM-DD'}
      title={invalid ? `请输入有效的${type === 'month' ? '月份' : '日期'}` : undefined}
      onChange={event => { const text = event.target.value; setDraft(text); setInvalid(false); const complete = type === 'month' ? /^\d{4}[-/.]\d{2}$/.test(text) : /^\d{4}[-/.]\d{2}[-/.]\d{2}$/.test(text); const normalized = normalize(text, type); if ((text === '' || complete) && normalized !== null && normalized !== value) onChange({ target: { value: normalized } }); }}
      onBlur={() => commit(draft)} onKeyDown={event => { if (event.key === 'Enter') { commit(draft); event.currentTarget.blur(); } }}/>
    <button ref={toggle} type="button" className="date-picker-button" aria-label={type === 'month' ? '选择月份' : '选择日期'} aria-expanded={open} aria-haspopup="dialog" disabled={disabled} onClick={show}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M7 3v4M17 3v4M3 10h18"/></svg>
    </button>
    {open && createPortal(<div ref={panel} className="date-calendar" role="dialog" aria-label={type === 'month' ? '月份选择面板' : '日期选择面板'} style={position}
      onMouseDown={event => event.stopPropagation()} onClick={event => event.stopPropagation()}
      onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); close(); } }}>
      <div className="calendar-heading"><button type="button" aria-label="上一年" disabled={view.year <= 1} onClick={() => setView({...view, year: view.year - 1})}>‹</button><strong>{view.year} 年</strong><button type="button" aria-label="下一年" disabled={view.year >= 9999} onClick={() => setView({...view, year: view.year + 1})}>›</button></div>
      {type === 'month' ? <div className="calendar-months">{Array.from({length: 12}, (_, index) => {
        const text = `${String(view.year).padStart(4, '0')}-${String(index + 1).padStart(2, '0')}`;
        return <button type="button" className="calendar-choice" key={index} aria-pressed={text === value} onClick={() => choose(text)}>{index + 1} 月</button>;
      })}</div> : <><div className="calendar-heading"><button type="button" aria-label="上个月" disabled={view.year === 1 && view.month === 1} onClick={() => setView(view.month === 1 ? {year: view.year - 1, month: 12} : {...view, month: view.month - 1})}>‹</button><strong>{view.month} 月</strong><button type="button" aria-label="下个月" disabled={view.year === 9999 && view.month === 12} onClick={() => setView(view.month === 12 ? {year: view.year + 1, month: 1} : {...view, month: view.month + 1})}>›</button></div><div className="calendar-days">{['一','二','三','四','五','六','日'].map(day => <span key={day}>{day}</span>)}{Array.from({length: offset}, (_, index) => <span key={`empty-${index}`}/>)}{Array.from({length: daysInMonth(view.year, view.month)}, (_, index) => {
        const text = `${prefix}-${String(index + 1).padStart(2, '0')}`;
        return <button type="button" className="calendar-choice" key={index} aria-label={text} aria-pressed={text === value} onClick={() => choose(text)}>{index + 1}</button>;
      })}</div></>}
      <div className="calendar-actions"><button type="button" onClick={() => choose('')}>清空</button><button type="button" onClick={close}>关闭</button></div>
    </div>, document.body)}
  </span>;
}
