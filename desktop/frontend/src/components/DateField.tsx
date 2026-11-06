import { useEffect, useRef, useState } from 'react';

type Props = {
  type: 'date' | 'month';
  value: string;
  onChange: (event: { target: { value: string } }) => void;
  disabled?: boolean;
};

function normalize(text: string, type: Props['type']): string | null {
  if (!text.trim()) return '';
  const parts = text.trim().match(/^(\d{4})[-/.年](\d{1,2})(?:[-/.月](\d{1,2})日?)?月?$/);
  if (!parts || (type === 'date') !== !!parts[3]) return null;
  const year = Number(parts[1]), month = Number(parts[2]), day = Number(parts[3]);
  if (year < 1 || month < 1 || month > 12) return null;
  if (type === 'date' && (day < 1 || day > new Date(year, month, 0).getDate())) return null;
  return `${parts[1]}-${String(month).padStart(2, '0')}${type === 'date' ? `-${String(day).padStart(2, '0')}` : ''}`;
}

export function DateField({ type, value, onChange, disabled }: Props) {
  const [draft, setDraft] = useState(value);
  const [invalid, setInvalid] = useState(false);
  const picker = useRef<HTMLInputElement>(null);
  useEffect(() => { setDraft(value); setInvalid(false); }, [value]);
  const commit = (text: string) => {
    const normalized = normalize(text, type);
    if (normalized === null) { setInvalid(true); return; }
    setDraft(normalized); setInvalid(false);
    if (normalized !== value) onChange({ target: { value: normalized } });
  };
  return <span className={`date-field${invalid ? ' invalid' : ''}`}>
    <input type="text" inputMode="numeric" value={draft} disabled={disabled} aria-invalid={invalid}
      aria-label={type === 'month' ? '输入月份' : '输入日期'} placeholder={type === 'month' ? 'YYYY-MM' : 'YYYY-MM-DD'}
      title={invalid ? `请输入有效的${type === 'month' ? '月份' : '日期'}` : undefined}
      onChange={event => { const text = event.target.value; setDraft(text); setInvalid(false); const complete = type === 'month' ? /^\d{4}[-/.]\d{2}$/.test(text) : /^\d{4}[-/.]\d{2}[-/.]\d{2}$/.test(text); const normalized = normalize(text, type); if ((text === '' || complete) && normalized !== null && normalized !== value) onChange({ target: { value: normalized } }); }}
      onBlur={() => commit(draft)} onKeyDown={event => { if (event.key === 'Enter') { commit(draft); event.currentTarget.blur(); } }}/>
    <input ref={picker} className="date-native-picker" type={type} value={value} tabIndex={-1} aria-hidden="true"
      onChange={event => commit(event.target.value)}/>
    <button type="button" className="date-picker-button" aria-label={type === 'month' ? '选择月份' : '选择日期'} disabled={disabled}
      onClick={() => { const input = picker.current; if (!input) return; if (input.showPicker) input.showPicker(); else input.click(); }}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M7 3v4M17 3v4M3 10h18"/></svg>
    </button>
  </span>;
}
