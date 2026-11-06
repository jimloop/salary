import { ComboSelect } from './ComboSelect';
import { cityRegions } from './cityRegions';

export function RegionSelect({ value, onChange, emptyLabel = '选择城市' }: {
  value: string;
  onChange: (event: { target: { value: string } }) => void;
  emptyLabel?: string;
}) {
  return <ComboSelect value={value} onChange={onChange}>
    <option value="">{emptyLabel}</option>
    {cityRegions.map(([city, label]) => <option key={city} value={city}>{label}</option>)}
  </ComboSelect>;
}
