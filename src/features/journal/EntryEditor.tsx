import { useRef, useState } from "react";
import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Placeholder } from "@tiptap/extensions";
import Image from "@tiptap/extension-image";
import type { JSONContent } from "@tiptap/core";
import { Bold, Heading2, ImagePlus, Italic, Link2, Minus, Quote, Redo2, Undo2 } from "lucide-react";
import { attachmentSource } from "../../lib/attachmentSource";
import { importAttachment } from "../../lib/entryApi";
import { errorText } from "../../lib/errorText";

type EntryEditorProps = {
  entryId: string;
  content: JSONContent;
  onChange: (content: JSONContent, contentText: string, coverAttachmentId: string | null) => void;
};

/** 正文里出现的附件 ID，按出现顺序排列；第一张就是日记首图。 */
function collectAttachmentIds(content: JSONContent): string[] {
  const ids: string[] = [];
  const walk = (node: JSONContent) => {
    if (node.type === "image" && typeof node.attrs?.attachmentId === "string") {
      ids.push(node.attrs.attachmentId);
    }
    node.content?.forEach(walk);
  };
  walk(content);
  return ids;
}

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
  Image.extend({
    addAttributes() {
      return {
        ...this.parent?.(),
        attachmentId: {
          default: null,
          parseHTML: (element) => element.getAttribute("data-attachment-id"),
          renderHTML: (attributes) =>
            attributes.attachmentId ? { "data-attachment-id": attributes.attachmentId } : {},
        },
      };
    },
  }).configure({ allowBase64: false }),
];

/**
 * 正文编辑器：结构化内容与纯文本副本一起回传，纯文本供摘要和搜索使用。
 * 切换日记时由调用方按 ID 重建实例，编辑器不负责跨日记保留状态。
 */
export function EntryEditor({ entryId, content, onChange }: EntryEditorProps) {
  const [initialContent] = useState(content);
  const [linkOpen, setLinkOpen] = useState(false);
  const [linkValue, setLinkValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  const editor = useEditor({
    extensions,
    content: initialContent,
    editorProps: {
      attributes: { class: "entry-editor", "aria-label": "日记正文" },
      handlePaste: (_view, event) => {
        const files = Array.from(event.clipboardData?.files ?? []);
        if (files.length === 0) return false;
        event.preventDefault();
        void insertImages(files);
        return true;
      },
      handleDrop: (_view, event) => {
        const files = Array.from(event.dataTransfer?.files ?? []);
        if (files.length === 0) return false;
        event.preventDefault();
        void insertImages(files);
        return true;
      },
    },
    onUpdate: ({ editor }) => {
      const json = editor.getJSON();
      onChange(json, editor.getText({ blockSeparator: "\n" }), collectAttachmentIds(json)[0] ?? null);
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

  /**
   * 先逐张落盘拿到附件 ID，再一次性插入所有成功的图片。
   * 分多次插入时后一张会替换前一张的选区，多图会只剩最后一张。
   */
  const insertImages = async (files: File[]) => {
    const nodes: JSONContent[] = [];
    let failed = false;
    for (const file of files) {
      try {
        const attachment = await importAttachment(entryId, new Uint8Array(await file.arrayBuffer()));
        nodes.push({
          type: "image",
          attrs: {
            src: attachmentSource(attachment.id),
            attachmentId: attachment.id,
            alt: file.name,
            width: attachment.width,
            height: attachment.height,
          },
        });
      } catch (cause) {
        failed = true;
        setError(errorText(cause));
      }
    }
    if (nodes.length === 0) return;
    editor.chain().focus().insertContent(nodes).run();
    if (!failed) setError(null);
  };

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
          className="editor-tool"
          type="button"
          title="插入图片"
          onClick={() => fileInput.current?.click()}
        >
          <ImagePlus size={15} strokeWidth={1.9} aria-hidden="true" />
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

      <input
        ref={fileInput}
        type="file"
        accept="image/png,image/jpeg"
        multiple
        hidden
        onChange={(event) => {
          const files = Array.from(event.target.files ?? []);
          event.target.value = "";
          void insertImages(files);
        }}
      />

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

      {error && (
        <p className="editor-error" role="alert">
          {error}
        </p>
      )}

      <EditorContent editor={editor} />
    </div>
  );
}
