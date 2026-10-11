import { el, button, badge } from './elements.js';
const links = [['overview','◈','Overview'],['agents','◉','Agents'],['integrations','⌘','Integrations'],
  ['policies','⛨','Policies'],['approvals','✓','Approvals'],['audit','≡','Activity & audit'],['security','⊙','Security'],['settings','⚙','Settings']];
export function shell(actor, logout) {
  const nav = el('nav', { 'aria-label': 'Main navigation' }, links.map(([id, icon, label]) =>
    el('a', { href: `#${id}`, 'data-page': id }, el('span', { class: 'nav-icon', 'aria-hidden': 'true' }, icon), label)));
  const sidebar = el('aside', { class: 'sidebar' }, el('a', { class: 'brand', href: '#overview' },
    el('span', { class: 'brand-icon' }, 'A'), el('div', {}, el('strong', {}, 'Agent Control'), el('small', {}, 'GOVERNANCE PLATFORM'))),
    el('div', { class: 'nav-caption' }, 'WORKSPACE'), nav,
    el('div', { class: 'sidebar-bottom' }, badge('Foundation', 'foundation'), el('p', {}, 'Independent control. Explicit permission.'),
    el('div', { class: 'user-row' }, el('span', { class: 'avatar' }, actor.username.slice(0,1).toUpperCase()),
      el('div', {}, el('strong', {}, actor.username), el('small', {}, actor.role)), button('Sign out', logout, 'text'))));
  const content = el('main', { class: 'content', id: 'content', tabindex: '-1' });
  const topbar = el('div', { class: 'topbar' }, el('span', {}, 'Organization / Control plane'),
    el('div', { class: 'topbar-status' }, el('span', { class: 'status-dot' }), 'Connected to local API', badge('v0.2')));
  return { node: el('div', { class: 'app-layout' }, el('a', { class: 'skip-link', href: '#content' }, 'Skip to content'), sidebar,
    el('div', { class: 'workspace' }, topbar, content)), content, nav };
}
