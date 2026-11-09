import { useEffect, useRef } from "react";
import { basicSetup } from "codemirror";
import { Compartment, EditorState, Prec } from "@codemirror/state";
import { EditorView, keymap, placeholder } from "@codemirror/view";
import { sql } from "@codemirror/lang-sql";
import type { SchemaMap } from "../types";

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
      color: "#dbe4ee",
      backgroundColor: "transparent",
      fontSize: "12px",
    },
    ".cm-content": {
      padding: "10px 0",
      caretColor: "#2dd4bf",
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
    },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "#55636f",
      border: "none",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(45, 212, 191, 0.05)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "transparent",
      color: "#2dd4bf",
    },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection": {
      backgroundColor: "rgba(45, 212, 191, 0.25)",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#2dd4bf",
    },
    ".cm-scroller": {
      overflow: "auto",
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
    },
    ".cm-tooltip": {
      backgroundColor: "#141a20",
      border: "1px solid #2a3744",
      color: "#dbe4ee",
    },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
      backgroundColor: "rgba(45, 212, 191, 0.15)",
      color: "#b8fff4",
    },
  },
  { dark: true },
);

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
