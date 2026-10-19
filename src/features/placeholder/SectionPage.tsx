import { pageTitles, type AppPage } from "../../app/navigation";

type PlaceholderPage = Exclude<
  AppPage,
  "today" | "timeline" | "calendar" | "search" | "letters" | "trash" | "settings"
>;

const sectionCopy: Record<PlaceholderPage, string> = {
  collection: "收藏那些不想忘记的小东西。",
};

export function SectionPage({ page }: { page: PlaceholderPage }) {
  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <h1 id="page-title">{pageTitles[page]}</h1>
        </div>
      </header>
      <div className="section-empty">
        <span>{sectionCopy[page]}</span>
      </div>
    </section>
  );
}
