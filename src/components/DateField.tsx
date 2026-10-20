import { useRef } from "react";
import { CalendarDays, X } from "lucide-react";
import { formatCalendarDate } from "../lib/localDate";

type DateFieldProps = {
  /** 字段说明，例如“从”“到”。 */
  label: string;
  /** 本地日历日期（YYYY-MM-DD）；空字符串表示还没选。 */
  value: string;
  onChange: (value: string) => void;
  /** 最早可选日期，通常是今天。 */
  min?: string;
  /** 空值时的提示文字。 */
  placeholder?: string;
  /** 有值时是否显示清除按钮。 */
  clearable?: boolean;
};

/**
 * 日期字段：外观是纸面上的一个日期，点开才出现系统日历。
 * 原生 date 输入只负责取值与唤起日历，界面文字始终用中文日期，
 * 不会出现 `yyyy/mm/日` 这种占位格式。
 */
export function DateField({ label, value, onChange, min, placeholder = "不限", clearable = true }: DateFieldProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  const openPicker = () => {
    const input = inputRef.current;
    if (!input) return;
    try {
      // showPicker 需要用户手势，按钮点击正好满足。
      input.showPicker();
    } catch {
      // 内核不支持或被拦截时退回聚焦，用户仍可键盘输入。
      input.focus();
    }
  };

  return (
    <span className="date-field">
      <span className="date-field-label">{label}</span>
      <span className="date-field-control">
        <button
          className={`date-field-chip${value ? " is-set" : ""}`}
          type="button"
          aria-haspopup="dialog"
          aria-label={`${label}：${value ? formatCalendarDate(value) : "未选择"}`}
          onClick={openPicker}
        >
          <CalendarDays size={13} strokeWidth={1.6} aria-hidden="true" />
          <span className="date-field-text">{value ? formatCalendarDate(value) : placeholder}</span>
        </button>
        {value && clearable && (
          <button
            className="date-field-clear"
            type="button"
            title="清除日期"
            aria-label={`清除${label}的日期`}
            onClick={() => onChange("")}
          >
            <X size={12} strokeWidth={2} aria-hidden="true" />
          </button>
        )}
        <input
          ref={inputRef}
          className="date-field-native"
          type="date"
          value={value}
          min={min}
          tabIndex={-1}
          aria-hidden="true"
          onChange={(event) => onChange(event.target.value)}
        />
      </span>
    </span>
  );
}
