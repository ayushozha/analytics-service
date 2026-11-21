// Stroke icons used by the sidebar (24×24 paths from the approved design).
export const ICONS = {
  home: 'M3 11l9-8 9 8M5 10v10h5v-6h4v6h5V10',
  insights: 'M4 20V10M10 20V4M16 20v-7M22 20H2',
  dashboards: 'M3 3h8v8H3zM13 3h8v5h-8zM13 10h8v11h-8zM3 13h8v8H3z',
  events: 'M13 2L4 14h7l-1 8 9-12h-7z',
  persons: 'M16 20v-1a4 4 0 00-4-4H8a4 4 0 00-4 4v1M10 11a4 4 0 100-8 4 4 0 000 8zM20 20v-1a4 4 0 00-3-3.9M15 3.1a4 4 0 010 7.8',
  groups: 'M4 21V5l8-3v19M12 21V9l8 3v9M2 21h20',
  cohorts: 'M12 3l9 5-9 5-9-5zM3 13l9 5 9-5',
  replay: 'M5 4l14 8-14 8z',
  flags: 'M5 21V4h11l-2 4 2 4H5',
  experiments: 'M9 3h6M10 3v6l-5 9a2 2 0 002 3h10a2 2 0 002-3l-5-9V3',
  surveys: 'M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z',
  errors: 'M12 9v4M12 17h.01M10.3 3.9L1.8 18a2 2 0 001.7 3h17a2 2 0 001.7-3L13.7 3.9a2 2 0 00-3.4 0z',
  logout: 'M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4M16 17l5-5-5-5M21 12H9',
} as const;

export type IconName = keyof typeof ICONS;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d={ICONS[name]} />
    </svg>
  );
}
