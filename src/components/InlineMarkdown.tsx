import { openUrl } from "@tauri-apps/plugin-opener";

// Emphasis requires non-space just inside the delimiters, so "a * b * c" stays literal.
const TOKEN =
  /(?<esc>\\[\\*`~[\]])|(?<code>`[^`]+`)|(?<bold>\*\*(?!\s)[^*]+(?<!\s)\*\*)|(?<strike>~~(?!\s)[^~]+(?<!\s)~~)|(?<italic>\*(?!\s)[^*]+(?<!\s)\*)|\[(?<linkText>[^\]]+)\]\((?<linkHref>[^)]+)\)/g;

function MarkdownLink({ href, text }: { href: string; text: string }) {
  return (
    <a
      href={href}
      // Opening in the webview would navigate away from the app; hand it to the OS instead.
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        openUrl(href).catch(() => {
          // not in Tauri context (browser dev), or a scheme the opener rejects
        });
      }}
      onDoubleClick={(e) => e.stopPropagation()}
      className="hover:text-blue-600 transition-colors"
      style={{ borderBottom: "1px solid rgba(0,0,0,0.2)" }}
    >
      {text}
    </a>
  );
}

function tokenNode(match: RegExpExecArray, key: number): React.ReactNode {
  const g = match.groups!;
  if (g.esc) return g.esc[1];
  if (g.code) {
    return (
      <code
        key={key}
        className="px-1 py-px rounded text-[0.85em]"
        style={{ fontFamily: "var(--font-mono, monospace)", background: "rgba(0,0,0,0.05)" }}
      >
        {g.code.slice(1, -1)}
      </code>
    );
  }
  if (g.bold) {
    return (
      <strong key={key} className="font-semibold">
        {g.bold.slice(2, -2)}
      </strong>
    );
  }
  if (g.strike) {
    return <del key={key}>{g.strike.slice(2, -2)}</del>;
  }
  if (g.italic) {
    return <em key={key}>{g.italic.slice(1, -1)}</em>;
  }
  return <MarkdownLink key={key} href={g.linkHref} text={g.linkText} />;
}

/**
 * Renders a single line of inline markdown as React nodes.
 *
 * Covers code spans, bold, italic, strikethrough, links and backslash escapes.
 * Block constructs and nested emphasis are not handled. Colors are inherited so
 * the caller's done/muted styling applies to the whole line.
 */
export function InlineMarkdown({ text }: { text: string }) {
  const nodes: React.ReactNode[] = [];
  let last = 0;

  for (const match of text.matchAll(TOKEN)) {
    if (match.index > last) nodes.push(text.slice(last, match.index));
    nodes.push(tokenNode(match, nodes.length));
    last = match.index + match[0].length;
  }
  if (last < text.length) nodes.push(text.slice(last));

  return <>{nodes}</>;
}
