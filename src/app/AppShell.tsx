import { BookHeart, ChevronRight } from "lucide-react";
import type { PropsWithChildren } from "react";
import type { AppPage, NavigationItem } from "./navigation";
import { primaryNavigation, utilityNavigation } from "./navigation";

type AppShellProps = PropsWithChildren<{
  activePage: AppPage;
  onNavigate: (page: AppPage) => void;
  /** 侧栏角标：到期待读的信有几封。 */
  letterBadge?: number;
}>;

function NavigationButton({
  item,
  active,
  badge,
  onClick,
}: {
  item: NavigationItem;
  active: boolean;
  badge?: number;
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
      {badge !== undefined && badge > 0 && (
        <span className="nav-badge" title={`${badge} 封到期的信`}>
          {badge > 9 ? "9+" : badge}
        </span>
      )}
      {active && <ChevronRight className="nav-chevron" aria-hidden="true" size={15} />}
    </button>
  );
}

export function AppShell({ activePage, onNavigate, letterBadge = 0, children }: AppShellProps) {
  return (
    <div className="app-shell">
      {/* 飘落的花瓣：纯装饰，不参与布局，读屏与键盘都跳过。 */}
      <div className="petals" aria-hidden="true">
        <span style={{ left: "8%", animationDuration: "13s", animationDelay: "0s" }} />
        <span style={{ left: "26%", animationDuration: "16s", animationDelay: "2s" }} />
        <span style={{ left: "47%", animationDuration: "14s", animationDelay: "5s" }} />
        <span style={{ left: "66%", animationDuration: "17s", animationDelay: "1s" }} />
        <span style={{ left: "82%", animationDuration: "12s", animationDelay: "4s" }} />
        <span style={{ left: "93%", animationDuration: "15s", animationDelay: "7s" }} />
      </div>

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
              badge={item.id === "letters" ? letterBadge : undefined}
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
