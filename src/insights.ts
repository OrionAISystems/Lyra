const ISO_DAY = /^(\d{4})-(\d{2})-(\d{2})$/;

export function formatUsageDay(value: string): string {
  const match = ISO_DAY.exec(value);
  if (!match) return value;

  const [, year, month, day] = match;
  const date = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)));
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(date);
}
