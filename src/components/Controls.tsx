import type { ReactNode } from "react";
import { ChevronDown, Info } from "lucide-react";
export function Page({
  title,
  subtitle,
  action,
  children,
}: {
  title: string;
  subtitle: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="page">
      <header className="page-head">
        <div>
          <h1>{title}</h1>
          <p>{subtitle}</p>
        </div>
        {action}
      </header>
      {children}
    </main>
  );
}
export function Card({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return <section className={`card ${className}`}>{children}</section>;
}
export function Row({
  icon,
  title,
  description,
  children,
}: {
  icon?: ReactNode;
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="setting-row">
      <div className="row-copy">
        {icon && <span className="row-icon">{icon}</span>}
        <div>
          <strong>{title}</strong>
          {description && <small>{description}</small>}
        </div>
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}
export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`toggle ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <span />
    </button>
  );
}
export function Select({
  value,
  onChange,
  children,
  label,
}: {
  value: string | number;
  onChange: (v: string) => void;
  children: ReactNode;
  label: string;
}) {
  return (
    <label className="select">
      <span className="sr-only">{label}</span>
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {children}
      </select>
      <ChevronDown size={14} />
    </label>
  );
}
export function Notice({ children }: { children: ReactNode }) {
  return (
    <div className="notice">
      <Info size={15} />
      <span>{children}</span>
    </div>
  );
}
export function Shortcut({ value }: { value: string }) {
  return (
    <span className="keys">
      {value.split("+").map((k, i) => (
        <span key={i}>
          <kbd>{k}</kbd>
          {i < value.split("+").length - 1 && <b>+</b>}
        </span>
      ))}
    </span>
  );
}
