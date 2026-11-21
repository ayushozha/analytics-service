import { NavLink } from 'react-router';
import { Icon, type IconName } from './Icon';

interface Item { icon: IconName; label: string; to?: string; end?: boolean }

// Items without `to` are on the roadmap and render as "Soon".
const SECTIONS: { label?: string; items: Item[] }[] = [
  { items: [
    { icon: 'home', label: 'Home', to: '/', end: true },
    { icon: 'insights', label: 'Insights', to: '/insights' },
    { icon: 'dashboards', label: 'Dashboards' },
  ] },
  { label: 'Data', items: [
    { icon: 'events', label: 'Events' },
    { icon: 'persons', label: 'Persons', to: '/persons' },
    { icon: 'groups', label: 'Groups' },
    { icon: 'cohorts', label: 'Cohorts' },
  ] },
  { label: 'Product', items: [
    { icon: 'replay', label: 'Session replay' },
    { icon: 'flags', label: 'Feature flags' },
    { icon: 'experiments', label: 'Experiments' },
    { icon: 'surveys', label: 'Surveys' },
    { icon: 'errors', label: 'Error tracking' },
  ] },
];

export function Sidebar({ projectName }: { projectName?: string }) {
  return (
    <nav className="sidebar" aria-label="Main">
      <div className="brand">
        <div className="brand-logo">
          <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff"
            strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="M2 12h4l3-8 6 16 3-8h4" /></svg>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <span className="brand-name">Pulse</span>
          <span className="brand-project">{projectName ?? ' '}</span>
        </div>
      </div>
      {SECTIONS.map((section, i) => (
        <div className="nav-section" key={i}>
          {section.label && <span className="nav-label">{section.label}</span>}
          {section.items.map((item) =>
            item.to ? (
              <NavLink key={item.label} to={item.to} end={item.end}
                className={({ isActive }) => `nav-item${isActive ? ' active' : ''}`}>
                <Icon name={item.icon} />
                <span>{item.label}</span>
              </NavLink>
            ) : (
              <span key={item.label} className="nav-item soon" aria-disabled="true">
                <Icon name={item.icon} />
                <span>{item.label}</span>
                <span className="soon-badge">Soon</span>
              </span>
            ),
          )}
        </div>
      ))}
      <form className="nav-foot" method="post" action="/dashboard/logout">
        <button className="nav-item" type="submit" style={{ font: 'inherit', border: 0, background: 'none', width: '100%', cursor: 'pointer' }}>
          <Icon name="logout" />
          <span>Sign out</span>
        </button>
      </form>
    </nav>
  );
}
