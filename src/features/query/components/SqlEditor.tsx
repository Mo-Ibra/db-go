import { useEffect, useRef } from "react";
import { basicSetup } from "codemirror";
import { Compartment, EditorState, Prec } from "@codemirror/state";
import { EditorView, keymap, placeholder } from "@codemirror/view";
import { sql } from "@codemirror/lang-sql";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";
import type { SchemaMap } from "@/types";

type SqlEditorProps = {
  value: string;
  schema: SchemaMap;
  onChange: (value: string) => void;
  onRun: (value: string) => void;
};

const theme = EditorView.theme(
  {
    "&": {
      height: "100%",
      color: "#e6edf3",
      backgroundColor: "#161d26",
      fontSize: "12px",
    },
    ".cm-content": {
      padding: "12px 0",
      caretColor: "#3fb950",
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
      lineHeight: "1.6",
    },
    ".cm-gutters": {
      backgroundColor: "#161d26",
      color: "#5c6773",
      border: "none",
      paddingRight: "8px",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(63, 185, 80, 0.08)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "transparent",
      color: "#3fb950",
    },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection": {
      backgroundColor: "rgba(63, 185, 80, 0.35)",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#3fb950",
    },
    ".cm-scroller": {
      overflow: "auto",
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
    },
    ".cm-tooltip": {
      backgroundColor: "#11161d",
      border: "1px solid #26303b",
      color: "#e6edf3",
      borderRadius: "6px",
    },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
      backgroundColor: "rgba(63, 185, 80, 0.2)",
      color: "#e6edf3",
    },
    ".cm-tooltip-autocomplete > ul > li": {
      padding: "4px 8px",
    },
    ".cm-panels": {
      backgroundColor: "#11161d",
      color: "#e6edf3",
    },
    ".cm-searchMatch": {
      backgroundColor: "rgba(187, 128, 9, 0.35)",
      outline: "1px solid rgba(187, 128, 9, 0.6)",
    },
  },
  { dark: true },
);

const highlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: "#f85149" },
  { tag: [tags.name, tags.deleted, tags.character, tags.propertyName, tags.macroName], color: "#79c0ff" },
  { tag: [tags.function(tags.variableName), tags.labelName], color: "#3fb950" },
  { tag: [tags.color, tags.constant(tags.name), tags.standard(tags.name)], color: "#79c0ff" },
  { tag: [tags.definition(tags.name), tags.separator], color: "#e6edf3" },
  { tag: [tags.typeName, tags.className, tags.number, tags.changed, tags.annotation, tags.modifier, tags.self, tags.namespace], color: "#ffa657" },
  { tag: [tags.operator, tags.operatorKeyword, tags.url, tags.escape, tags.regexp, tags.link], color: "#f85149" },
  { tag: [tags.meta, tags.comment], color: "#8b949e", fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "bold" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: tags.link, color: "#8b949e", textDecoration: "underline" },
  { tag: tags.heading, fontWeight: "bold", color: "#79c0ff" },
  { tag: [tags.atom, tags.bool, tags.special(tags.variableName)], color: "#79c0ff" },
  { tag: [tags.string, tags.inserted], color: "#a5d6ff" },
  { tag: tags.invalid, color: "#f85149" },
]);

export function SqlEditor({ value, schema, onChange, onRun }: SqlEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const onRunRef = useRef(onRun);
  const language = useRef(new Compartment());

  onChangeRef.current = onChange;
  onRunRef.current = onRun;

  useEffect(() => {
    if (!host.current) return;

    const state = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        theme,
        syntaxHighlighting(highlightStyle),
        EditorView.lineWrapping,
        placeholder("-- Write a read-only SQL query"),
        Prec.highest(
          keymap.of([
            {
              key: "Mod-Enter",
              run: (editorView) => {
                onRunRef.current(editorView.state.doc.toString());
                return true;
              },
            },
          ]),
        ),
        language.current.of(
          sql({
            schema,
            upperCaseKeywords: true,
          }),
        ),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChangeRef.current(update.state.doc.toString());
          }
        }),
      ],
    });

    const instance = new EditorView({ state, parent: host.current });
    view.current = instance;
    instance.focus();

    return () => {
      instance.destroy();
      view.current = null;
    };
    // The editor is created once; value/schema updates are handled below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;

    const current = instance.state.doc.toString();
    if (current !== value) {
      instance.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
    }
  }, [value]);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;

    instance.dispatch({
      effects: language.current.reconfigure(
        sql({
          schema,
          upperCaseKeywords: true,
        }),
      ),
    });
  }, [schema]);

  return <div className="h-full" ref={host} />;
}
