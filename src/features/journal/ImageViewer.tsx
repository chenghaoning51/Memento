import { useEffect } from "react";
import { Trash2, X } from "lucide-react";

type ImageViewerProps = {
  /** 原图地址；为空时不显示。 */
  src: string | null;
  alt?: string;
  onClose: () => void;
  onDelete: () => void;
};

/**
 * 图片放大：正文里的图片按纸张宽度显示，想看细节时点开这一层。
 * 覆盖整窗，点背景或按 Esc 关闭；删除按钮把图片从正文与磁盘一起移除。
 */
export function ImageViewer({ src, alt = "", onClose, onDelete }: ImageViewerProps) {
  useEffect(() => {
    if (!src) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [src, onClose]);

  if (!src) return null;

  return (
    <div className="image-viewer" role="dialog" aria-modal="true" aria-label="查看图片" onClick={onClose}>
      <div className="image-viewer-frame" onClick={(event) => event.stopPropagation()}>
        <img className="image-viewer-image" src={src} alt={alt} />
        <div className="image-viewer-actions">
          <button className="image-viewer-button" type="button" onClick={onDelete} title="删除这张图片">
            <Trash2 size={14} strokeWidth={1.8} aria-hidden="true" />
            删除这张图片
          </button>
          <button className="image-viewer-button" type="button" onClick={onClose} title="关闭">
            <X size={14} strokeWidth={1.9} aria-hidden="true" />
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
