import { useCallback, useEffect, useMemo, useState } from "react";
import type { ColumnInfo } from "@/types";
import type { CardLayout, CardTemplate, CardTemplateStore } from "@/features/cards/model";
import {
  buildTemplates,
  cardTemplateStorageKey,
  loadTemplateStore,
  saveTemplateStore,
} from "@/features/cards/templates";

type UseCardTemplatesOptions = {
  dbPath: string | null;
  table: string | null;
  columns: ColumnInfo[];
};

export function useCardTemplates({
  dbPath,
  table,
  columns,
}: UseCardTemplatesOptions) {
  const [store, setStore] = useState<CardTemplateStore | null>(null);
  const [activeTemplateId, setActiveTemplateId] = useState<string | null>(null);

  const storageKey = useMemo(
    () => (dbPath && table ? cardTemplateStorageKey(dbPath, table) : null),
    [dbPath, table],
  );

  useEffect(() => {
    if (!table || !storageKey || columns.length === 0) {
      setStore(null);
      setActiveTemplateId(null);
      return;
    }

    const existing = loadTemplateStore(storageKey);
    const next = existing ?? buildTemplates(table, columns);
    setStore(next);
    const preferred =
      next.templates.find((template) => template.id === next.defaultTemplateId) ??
      next.templates[0] ??
      null;
    setActiveTemplateId(preferred?.id ?? null);
    if (!existing) saveTemplateStore(storageKey, next);
  }, [table, storageKey, columns]);

  const selectTemplate = useCallback(
    (templateId: string) => {
      setActiveTemplateId(templateId);
      setStore((current) => {
        if (!current || !storageKey) return current;
        const next = { ...current, defaultTemplateId: templateId };
        saveTemplateStore(storageKey, next);
        return next;
      });
    },
    [storageKey],
  );

  const activeTemplate =
    store?.templates.find((template) => template.id === activeTemplateId) ??
    store?.templates[0] ??
    null;

  const commit = useCallback(
    (updater: (current: CardTemplateStore) => CardTemplateStore) => {
      setStore((current) => {
        if (!current || !storageKey) return current;
        const next = updater(current);
        saveTemplateStore(storageKey, next);
        return next;
      });
    },
    [storageKey],
  );

  const updateTemplate = useCallback(
    (template: CardTemplate) => {
      commit((current) => ({
        ...current,
        templates: current.templates.map((item) =>
          item.id === template.id ? template : item,
        ),
      }));
    },
    [commit],
  );

  const renameTemplate = useCallback(
    (templateId: string, name: string) => {
      commit((current) => ({
        ...current,
        templates: current.templates.map((item) =>
          item.id === templateId ? { ...item, name } : item,
        ),
      }));
    },
    [commit],
  );

  const changeLayout = useCallback(
    (templateId: string, layout: CardLayout) => {
      if (!table || columns.length === 0) return;
      const prototype = buildTemplates(table, columns).templates.find(
        (item) => item.layout === layout,
      );
      if (!prototype) return;
      commit((current) => ({
        ...current,
        templates: current.templates.map((item) =>
          item.id === templateId
            ? { ...item, layout, card: prototype.card }
            : item,
        ),
      }));
    },
    [columns, commit, table],
  );

  const createTemplate = useCallback(
    (layout: CardLayout = "article") => {
      if (!table || columns.length === 0) return;
      const prototype =
        buildTemplates(table, columns).templates.find(
          (item) => item.layout === layout,
        ) ?? buildTemplates(table, columns).templates[0];
      if (!prototype) return;
      let name = prototype.name;
      let suffix = 2;
      const existingNames = new Set(store?.templates.map((item) => item.name) ?? []);
      while (existingNames.has(name)) {
        name = `${prototype.name} ${suffix++}`;
      }
      const template: CardTemplate = {
        ...prototype,
        id: `${table}-${layout}-${Date.now()}`,
        name,
      };
      commit((current) => ({
        ...current,
        defaultTemplateId: template.id,
        templates: [...current.templates, template],
      }));
      setActiveTemplateId(template.id);
    },
    [columns, commit, store?.templates, table],
  );

  const duplicateTemplate = useCallback(
    (templateId: string) => {
      const source = store?.templates.find((item) => item.id === templateId);
      if (!source) return;
      let name = `${source.name} copy`;
      let suffix = 2;
      const existingNames = new Set(store?.templates.map((item) => item.name) ?? []);
      while (existingNames.has(name)) {
        name = `${source.name} copy ${suffix++}`;
      }
      const template: CardTemplate = {
        ...source,
        id: `${source.table}-${source.layout}-${Date.now()}`,
        name,
      };
      commit((current) => ({
        ...current,
        defaultTemplateId: template.id,
        templates: [...current.templates, template],
      }));
      setActiveTemplateId(template.id);
    },
    [commit, store?.templates],
  );

  const deleteTemplate = useCallback(
    (templateId: string) => {
      if (!store || store.templates.length <= 1) return;
      const remaining = store.templates.filter((item) => item.id !== templateId);
      const nextDefault =
        store.defaultTemplateId === templateId
          ? remaining[0]?.id ?? null
          : store.defaultTemplateId;
      commit((current) => ({
        ...current,
        defaultTemplateId: nextDefault,
        templates: remaining,
      }));
      if (activeTemplateId === templateId) {
        setActiveTemplateId(nextDefault);
      }
    },
    [activeTemplateId, commit, store],
  );

  return {
    templates: store?.templates ?? [],
    activeTemplate,
    activeTemplateId,
    selectTemplate,
    updateTemplate,
    renameTemplate,
    changeLayout,
    createTemplate,
    duplicateTemplate,
    deleteTemplate,
  };
}
