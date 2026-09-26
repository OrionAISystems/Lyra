import {
  Children,
  isValidElement,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactElement,
  type ReactNode,
} from "react";
import { Check, ChevronDown, Info } from "lucide-react";
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
  className = "",
}: {
  icon?: ReactNode;
  title: string;
  description?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={`setting-row ${className}`}>
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
  disabled = false,
}: {
  value: string | number;
  onChange: (v: string) => void;
  children: ReactNode;
  label: string;
  disabled?: boolean;
}) {
  type OptionProps = {
    value?: string | number;
    children?: ReactNode;
    disabled?: boolean;
  };
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [highlighted, setHighlighted] = useState(0);
  const options = Children.toArray(children)
    .filter(isValidElement)
    .map((child) => {
      const props = (child as ReactElement<OptionProps>).props;
      return {
        value: String(props.value ?? ""),
        label: String(props.children ?? ""),
        disabled: Boolean(props.disabled),
      };
    });
  const selectedValue = String(value);
  const selectedIndex = Math.max(
    0,
    options.findIndex((option) => option.value === selectedValue),
  );
  const selected = options[selectedIndex] ?? options[0];

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const moveHighlight = (direction: 1 | -1) => {
    if (!options.length) return;
    let next = highlighted;
    for (let i = 0; i < options.length; i += 1) {
      next = (next + direction + options.length) % options.length;
      if (!options[next]?.disabled) break;
    }
    setHighlighted(next);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) {
        setHighlighted(selectedIndex);
        setOpen(true);
      } else {
        moveHighlight(event.key === "ArrowDown" ? 1 : -1);
      }
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (!open) {
        setHighlighted(selectedIndex);
        setOpen(true);
      } else if (!options[highlighted]?.disabled) {
        onChange(options[highlighted].value);
        setOpen(false);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
    }
  };

  return (
    <div ref={rootRef} className={`select ${open ? "open" : ""}`}>
      <span className="sr-only" id={`${label.replaceAll(" ", "-")}-label`}>
        {label}
      </span>
      <button
        type="button"
        className="select-trigger"
        disabled={disabled}
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={`${label.replaceAll(" ", "-")}-menu`}
        onClick={() => {
          setHighlighted(selectedIndex);
          setOpen((current) => !current);
        }}
        onKeyDown={handleKeyDown}
      >
        <span>{selected?.label ?? "Select…"}</span>
        <ChevronDown size={14} aria-hidden="true" />
      </button>
      {open && (
        <div
          className="select-menu"
          id={`${label.replaceAll(" ", "-")}-menu`}
          role="listbox"
          aria-label={label}
        >
          {options.map((option, index) => (
            <button
              type="button"
              role="option"
              aria-selected={option.value === selectedValue}
              disabled={option.disabled}
              className={`select-option ${
                option.value === selectedValue ? "selected" : ""
              } ${index === highlighted ? "highlighted" : ""}`}
              key={option.value}
              onMouseEnter={() => setHighlighted(index)}
              onClick={() => {
                if (option.disabled) return;
                onChange(option.value);
                setOpen(false);
              }}
            >
              <span>{option.label}</span>
              {option.value === selectedValue && <Check size={14} />}
            </button>
          ))}
        </div>
      )}
    </div>
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
