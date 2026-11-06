import { Children, isValidElement, useId, useState } from 'react';
import type { ReactNode } from 'react';

type Option = { value: string; label: string; disabled: boolean };
type Props = {
  value?: string | number;
  defaultValue?: string | number;
  onChange?: (event: { target: { value: string } }) => void;
  disabled?: boolean;
  children: ReactNode;
  'aria-label'?: string;
};

function textOf(value: ReactNode): string {
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  return Children.toArray(value).map(child => isValidElement<{ children?: ReactNode }>(child) ? textOf(child.props.children) : textOf(child)).join('');
}

export function ComboSelect({ value, defaultValue, onChange, disabled, children, 'aria-label': ariaLabel }: Props) {
  const [localValue, setLocalValue] = useState(String(defaultValue ?? ''));
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const listId = useId();
  const options: Option[] = Children.toArray(children).filter(isValidElement).filter(child => child.type === 'option').map(child => {
    const props = child.props as { value?: string | number; children?: ReactNode; disabled?: boolean };
    const label = textOf(props.children);
    return { value: String(props.value ?? label), label, disabled: !!props.disabled };
  });
  const selected = String(value ?? localValue);
  const selectedLabel = options.find(option => option.value === selected)?.label ?? '';
  const matches = options.filter(option => !option.disabled && (option.label.toLocaleLowerCase().includes(query.toLocaleLowerCase()) || option.value.toLocaleLowerCase().includes(query.toLocaleLowerCase())));
  const choose = (option: Option, close = true) => {
    if (value === undefined) setLocalValue(option.value);
    if (option.value !== selected) onChange?.({ target: { value: option.value } });
    if (close) { setOpen(false); setQuery(''); }
  };
  return <div className={`combo-select${disabled ? ' disabled' : ''}`}>
    <input role="combobox" type="text" aria-label={ariaLabel} aria-autocomplete="list" aria-expanded={open} aria-controls={listId}
      aria-activedescendant={open && matches[active] ? `${listId}-${active}` : undefined}
      disabled={disabled} value={open ? query : selectedLabel} placeholder={open ? selectedLabel : '请选择'} autoComplete="off"
      onFocus={() => { setOpen(true); setQuery(''); setActive(-1); }}
      onChange={event => {
        const typed = event.target.value;
        setQuery(typed); setOpen(true); setActive(-1);
        const exact = options.find(option => !option.disabled && (option.label.toLocaleLowerCase() === typed.toLocaleLowerCase() || option.value.toLocaleLowerCase() === typed.toLocaleLowerCase()));
        if (exact) choose(exact, false);
      }}
      onBlur={() => { if (query && matches.length === 1) choose(matches[0]); else { setOpen(false); setQuery(''); } }}
      onKeyDown={event => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault(); setOpen(true); setActive(current => Math.max(0, Math.min(matches.length - 1, current + (event.key === 'ArrowDown' ? 1 : -1))));
        } else if (event.key === 'Enter' && open) {
          event.preventDefault(); if (matches[active] || matches.length === 1) choose(matches[active] ?? matches[0]);
        } else if (event.key === 'Escape') { setOpen(false); setQuery(''); }
      }}/>
    <button type="button" className="combo-trigger" tabIndex={-1} aria-label="展开选项" disabled={disabled} onMouseDown={event => event.preventDefault()} onClick={event => {
      const input = event.currentTarget.previousElementSibling as HTMLInputElement;
      input.focus(); setOpen(!open); setQuery(''); setActive(-1);
    }}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></button>
    {open && !disabled && <div className="combo-options" id={listId} role="listbox">
      {matches.length ? matches.map((option, index) => <button type="button" role="option" id={`${listId}-${index}`} aria-selected={option.value === selected}
        className={index === active ? 'active' : ''} key={`${option.value}-${index}`} onMouseDown={event => event.preventDefault()} onClick={() => choose(option)}>{option.label}</button>) : <div className="combo-empty">无匹配选项</div>}
    </div>}
  </div>;
}
