/** Tiny, safe Markdown renderer for the bundled tool docs (headings, paragraphs, lists, bold, code, links). */
const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

function inline(s: string): string {
  return esc(s)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\*([^*]+)\*/g, "<em>$1</em>")
    // Links are shown as text + visible URL; the app never navigates away from itself.
    .replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '$1 (<code>$2</code>)');
}

export interface TermMatcher {
  words: string[];
  definition: string;
}

const escRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Wraps the first mention of each glossary term in a tooltip. Code and existing tags are left alone. */
function annotate(html: string, terms: TermMatcher[]): string {
  const done = new Set<number>();
  let inCode = false;
  return html
    .split(/(<[^>]+>)/)
    .map((part) => {
      if (part.startsWith("<")) {
        if (part === "<code>") inCode = true;
        if (part === "</code>") inCode = false;
        return part;
      }
      if (inCode) return part;
      let out = part;
      terms.forEach((t, i) => {
        if (done.has(i) || !t.words.length) return;
        const re = new RegExp(`(^|[^\\p{L}\\p{N}])(${t.words.map(escRe).join("|")})(?![\\p{L}\\p{N}])`, "iu");
        if (re.test(out)) {
          done.add(i);
          out = out.replace(re, (_m, pre: string, word: string) => `${pre}<abbr class="term" tabindex="0" title="${esc(t.definition)}">${word}</abbr>`);
        }
      });
      return out;
    })
    .join("");
}

export function renderMarkdown(src: string, terms: TermMatcher[] = []): string {
  const out: string[] = [];
  let list = false;
  const closeList = () => {
    if (list) out.push("</ul>");
    list = false;
  };
  for (const raw of src.split(/\r?\n/)) {
    const line = raw.trimEnd();
    const h = /^(#{1,3})\s+(.*)$/.exec(line);
    if (h) {
      closeList();
      out.push(`<h${h[1].length + 1}>${inline(h[2])}</h${h[1].length + 1}>`);
    } else if (/^[-*]\s+/.test(line)) {
      if (!list) out.push("<ul>");
      list = true;
      out.push(`<li>${inline(line.replace(/^[-*]\s+/, ""))}</li>`);
    } else if (line === "") {
      closeList();
    } else {
      closeList();
      out.push(`<p>${inline(line)}</p>`);
    }
  }
  closeList();
  const html = out.join("\n");
  return terms.length ? annotate(html, terms) : html;
}
