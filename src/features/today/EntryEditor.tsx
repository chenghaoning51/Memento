import { useState } from "react";
import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Placeholder } from "@tiptap/extensions";
import type { JSONContent } from "@tiptap/core";
import { Bold, Heading2, Italic, Link2, Minus, Quote, Redo2, Undo2 } from "lucide-react";

type EntryEditorProps = {
  content: JSONContent;
  onChange: (content: JSONContent, contentText: string) => void;
};

const extensions = [
  StarterKit.configure({
    code: false,
    codeBlock: false,
    bulletList: false,
    orderedList: false,
    listItem: false,
    listKeymap: false,
    strike: false,
    underline: false,
    link: {
      openOnClick: false,
      autolink: true,
      HTMLAttributes: { rel: "noopener noreferrer" },
    },
  }),
  Placeholder.configure({ placeholder: "今天发生了什么？" }),
];

/**
 * 正文编辑器：结构化内容与纯文本副本一起回传，纯文本供摘要和搜索使用。
 * 切换日记时由调用方按 ID 重建实例，编辑器不负责跨日记保留状态。
 */
export function EntryEditor({ content, onChange }: EntryEditorProps) {
  const [initialContent] = useState(content);
  const [linkOpen, setLinkOpen] = useState(false);
  const [linkValue, setLinkValue] = useState("");

  const editor = useEditor({
    extensions,
    content: initialContent,
    editorProps: { attributes: { class: "entry-editor", "aria-label": "日记正文" } },
    onUpdate: ({ editor }) => {
      onChange(editor.getJSON(), editor.getText({ blockSeparator: "\n" }));
    },
  });

  const active = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor.isActive("bold"),
      italic: editor.isActive("italic"),
      heading: editor.isActive("heading", { level: 2 }),
      quote: editor.isActive("blockquote"),
      link: editor.isActive("link"),
      canUndo: editor.can().undo(),
      canRedo: editor.can().redo(),
    }),
  });

  if (!editor) return null;

  const openLink = () => {
    const href = editor.getAttributes("link").href as string | undefined;
    setLinkValue(href ?? "");
    setLinkOpen(true);
  };

  const applyLink = () => {
    const href = linkValue.trim();
    if (!href) return;
    editor.chain().focus().extendMarkRange("link").setLink({ href }).run();
    setLinkOpen(false);
    setLinkValue("");
  };

  const removeLink = () => {
    editor.chain().focus().extendMarkRange("link").unsetLink().run();
    setLinkOpen(false);
    setLinkValue("");
  };

  return (
    <div className="editor-shell">
      <div className="editor-toolbar" role="toolbar" aria-label="正文格式">
        <button
          className={`editor-tool${active.bold ? " is-active" : ""}`}
          type="button"
          title="加粗"
          aria-pressed={active.bold}
          onClick={() => editor.chain().focus().toggleBold().run()}
        >
          <Bold size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button
          className={`editor-tool${active.italic ? " is-active" : ""}`}
          type="button"
          title="斜体"
          aria-pressed={active.italic}
          onClick={() => editor.chain().focus().toggleItalic().run()}
        >
          <Italic size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button
          className={`editor-tool${active.heading ? " is-active" : ""}`}
          type="button"
          title="小标题"
          aria-pressed={active.heading}
          onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        >
          <Heading2 size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button
          className={`editor-tool${active.quote ? " is-active" : ""}`}
          type="button"
          title="引用"
          aria-pressed={active.quote}
          onClick={() => editor.chain().focus().toggleBlockquote().run()}
        >
          <Quote size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button
          className="editor-tool"
          type="button"
          title="分隔线"
          onClick={() => editor.chain().focus().setHorizontalRule().run()}
        >
          <Minus size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button
          className={`editor-tool${active.link ? " is-active" : ""}`}
          type="button"
          title="链接"
          aria-pressed={active.link}
          onClick={openLink}
        >
          <Link2 size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <span className="editor-tool-divider" aria-hidden="true" />
        <button
          className="editor-tool"
          type="button"
          title="撤销"
          disabled={!active.canUndo}
          onClick={() => editor.chain().focus().undo().run()}
        >
          <Undo2 size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button
          className="editor-tool"
          type="button"
          title="重做"
          disabled={!active.canRedo}
          onClick={() => editor.chain().focus().redo().run()}
        >
          <Redo2 size={15} strokeWidth={1.9} aria-hidden="true" />
        </button>
      </div>

      {linkOpen && (
        <div className="editor-link-row">
          <input
            className="editor-link-input"
            value={linkValue}
            onChange={(event) => setLinkValue(event.target.value)}
            placeholder="https://example.com"
            aria-label="链接地址"
            onKeyDown={(event) => {
              if (event.key === "Enter") applyLink();
              if (event.key === "Escape") setLinkOpen(false);
            }}
          />
          <button className="link-button" type="button" onClick={applyLink}>
            确定
          </button>
          {active.link && (
            <button className="link-button" type="button" onClick={removeLink}>
              移除
            </button>
          )}
        </div>
      )}

      <EditorContent editor={editor} />
    </div>
  );
}
