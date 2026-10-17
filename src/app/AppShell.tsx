import { BookHeart, ChevronRight } from "lucide-react";
import type { PropsWithChildren } from "react";
import type { AppPage, NavigationItem } from "./navigation";
import { primaryNavigation, utilityNavigation } from "./navigation";

type AppShellProps = PropsWithChildren<{
  activePage: AppPage;
  onNavigate: (page: AppPage) => void;
}>;

function NavigationButton({
  item,
  active,
  onClick,
}: {
  item: NavigationItem;
  active: boolean;
  onClick: () => void;
}) {
  const Icon = item.icon;

  return (
    <button
      className={`nav-item${active ? " is-active" : ""}`}
      type="button"
      aria-current={active ? "page" : undefined}
      onClick={onClick}
      title={item.label}
    >
      <Icon aria-hidden="true" size={18} strokeWidth={1.7} />
      <span>{item.label}</span>
      {active && <ChevronRight className="nav-chevron" aria-hidden="true" size={15} />}
    </button>
  );
}

export function AppShell({ activePage, onNavigate, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="主导航">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">
            <BookHeart size={22} strokeWidth={1.7} />
          </div>
          <div className="brand-copy">
            <span className="brand-name">Memento</span>
            <span className="brand-caption">私人时光收藏室</span>
          </div>
        </div>

        <div className="sidebar-rule" />
        <p className="nav-heading">我的空间</p>
        <nav className="primary-nav" aria-label="内容">
          {primaryNavigation.map((item) => (
            <NavigationButton
              key={item.id}
              item={item}
              active={activePage === item.id}
              onClick={() => onNavigate(item.id)}
            />
          ))}
        </nav>

        <div className="sidebar-bottom">
          <div className="sidebar-rule" />
          {utilityNavigation.map((item) => (
            <NavigationButton
              key={item.id}
              item={item}
              active={activePage === item.id}
              onClick={() => onNavigate(item.id)}
            />
          ))}
          <p className="sidebar-footnote">留给自己的时间</p>
        </div>
      </aside>

      <main className="main-panel">
        <div className="page-content">{children}</div>
      </main>
    </div>
  );
}
