import { pageTitles, type AppPage } from "../../app/navigation";

type PlaceholderPage = Exclude<
  AppPage,
  "today" | "timeline" | "calendar" | "search" | "trash" | "settings"
>;

const sectionCopy: Record<PlaceholderPage, string> = {
  letters: "写给未来的信，会在约定的日子打开。",
  collection: "收藏那些不想忘记的小东西。",
};

export function SectionPage({ page }: { page: PlaceholderPage }) {
  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">MEMENTO ARCHIVE</p>
          <h1 id="page-title">{pageTitles[page]}</h1>
        </div>
      </header>
      <div className="section-empty">
        <span>{sectionCopy[page]}</span>
      </div>
    </section>
  );
}
