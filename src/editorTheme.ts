import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

const tickBaseTheme = EditorView.theme(
  {
    "&": {
      backgroundColor: "#fbfdf9",
      color: "#18231f",
    },
    ".cm-content": {
      caretColor: "#16836f",
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace',
      fontSize: "13px",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#16836f",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "rgba(22, 131, 111, 0.18)",
    },
    ".cm-panels": {
      backgroundColor: "#f4f8f4",
      color: "#18231f",
    },
    ".cm-gutters": {
      backgroundColor: "#eef4ef",
      borderRight: "1px solid rgba(22, 131, 111, 0.12)",
      color: "#7c8984",
    },
    ".cm-activeLine, .cm-activeLineGutter": {
      backgroundColor: "rgba(22, 131, 111, 0.07)",
    },
    ".cm-foldPlaceholder": {
      backgroundColor: "rgba(22, 131, 111, 0.1)",
      borderColor: "rgba(22, 131, 111, 0.2)",
      color: "#16836f",
    },
  },
  { dark: true },
);

const tickHighlightStyle = HighlightStyle.define([
  { tag: [t.keyword, t.operatorKeyword], color: "#16836f" },
  { tag: [t.name, t.deleted, t.character, t.propertyName, t.macroName], color: "#18231f" },
  { tag: [t.function(t.variableName), t.labelName], color: "#0e7191" },
  { tag: [t.color, t.constant(t.name), t.standard(t.name)], color: "#9a6217" },
  { tag: [t.definition(t.name), t.separator], color: "#18231f" },
  { tag: [t.typeName, t.className, t.number, t.changed, t.annotation, t.modifier, t.self, t.namespace], color: "#7048a8" },
  { tag: [t.operator, t.operatorKeyword], color: "#16836f" },
  { tag: [t.url, t.escape, t.regexp, t.link], color: "#0e7191" },
  { tag: [t.meta, t.comment], color: "#77847f" },
  { tag: t.strong, fontWeight: "700" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strikethrough, textDecoration: "line-through" },
  { tag: t.link, color: "#0e7191", textDecoration: "underline" },
  { tag: t.heading, fontWeight: "700", color: "#10231d" },
  { tag: [t.atom, t.bool, t.special(t.variableName)], color: "#9a6217" },
  { tag: [t.processingInstruction, t.string, t.inserted], color: "#b14f2f" },
  { tag: t.invalid, color: "#c73d35" },
]);

export const tickEditorTheme = [tickBaseTheme, syntaxHighlighting(tickHighlightStyle)];
