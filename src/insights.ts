const ISO_DAY = /^(\d{4})-(\d{2})-(\d{2})$/;
const ISO_MONTH = /^(\d{4})-(\d{2})$/;

export function formatUsageDay(value: string): string {
  if (/^\d{4}$/.test(value)) return value;
  const monthMatch = ISO_MONTH.exec(value);
  if (monthMatch) {
    const [, year, month] = monthMatch;
    const date = new Date(Date.UTC(Number(year), Number(month) - 1, 1));
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      year: "2-digit",
      timeZone: "UTC",
    }).format(date);
  }
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
