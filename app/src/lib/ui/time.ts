const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * "just now" / "12 minutes ago" / "3 hours ago". Used for game start and last
 * activity times, which are always in the recent past.
 */
export function timeAgo(iso: string): string {
	const then = Date.parse(iso);
	if (Number.isNaN(then)) return '';
	const secs = Math.max(0, Math.round((Date.now() - then) / 1000));

	if (secs < 45) return 'just now';
	if (secs < HOUR) return plural(Math.round(secs / MINUTE), 'minute');
	if (secs < DAY) return plural(Math.round(secs / HOUR), 'hour');
	return plural(Math.round(secs / DAY), 'day');
}

function plural(n: number, unit: string): string {
	return `${n} ${unit}${n === 1 ? '' : 's'} ago`;
}
