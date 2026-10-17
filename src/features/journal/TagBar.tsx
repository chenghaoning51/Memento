import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { addEntryTag, listEntryTags, removeEntryTag } from "../../lib/entryApi";
import type { Tag } from "../../types/entry";

/** 标签栏：确认或移除都立刻写库，不等正文的自动保存。 */
export function TagBar({ entryId }: { entryId: string }) {
  const [tags, setTags] = useState<Tag[]>([]);
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    listEntryTags(entryId)
      .then(setTags)
      .catch((cause: unknown) => setError(String(cause)));
  }, [entryId]);

  const commit = async () => {
    const name = value.trim();
    if (!name) return;
    try {
      setTags(await addEntryTag(entryId, name));
      setValue("");
      setError(null);
    } catch (cause) {
      setError(String(cause));
    }
  };

  const remove = async (tagId: string) => {
    try {
      setTags(await removeEntryTag(entryId, tagId));
      setError(null);
    } catch (cause) {
      setError(String(cause));
    }
  };

  return (
    <div className="tag-bar">
      <div className="tag-chips">
        {tags.map((tag) => (
          <span className="tag-chip" key={tag.id}>
            {tag.name}
            <button
              className="tag-remove"
              type="button"
              title={`移除标签 ${tag.name}`}
              onClick={() => void remove(tag.id)}
            >
              <X size={11} strokeWidth={2.2} aria-hidden="true" />
            </button>
          </span>
        ))}
        <input
          ref={input}
          className="tag-input"
          value={value}
          placeholder={tags.length === 0 ? "添加标签" : "继续添加"}
          aria-label="添加标签"
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === ",") {
              event.preventDefault();
              void commit();
            }
          }}
        />
      </div>
      {error && (
        <p className="editor-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
