import { useCallback, useEffect, useRef, useState } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Edge,
  type Node,
  type NodeProps,
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import dagre from "@dagrejs/dagre";
import type { SchemaGraph, SchemaTable } from "../types";

type TableNodeData = {
  table: SchemaTable;
  onOpenTable: (table: string) => void;
};

type TableNodeType = Node<TableNodeData, "table">;

type DiagramViewProps = {
  graph: SchemaGraph | null;
  dbPath: string;
  onOpenTable: (table: string) => void;
  onCreateTable: () => void;
};

type StoredPositions = Record<string, { x: number; y: number }>;

const NODE_WIDTH = 260;
const HEADER_HEIGHT = 34;
const ROW_HEIGHT = 26;

const BUTTON =
  "inline-flex h-7 items-center justify-center rounded-md border border-line bg-panel-2 px-2.5 text-[11px] font-medium text-fg transition-colors hover:border-line-strong hover:bg-panel-3";
const INPUT =
  "h-7 w-[190px] rounded-md border border-line bg-ink px-2 text-[11px] text-fg outline-none placeholder:text-muted-2 focus:border-accent";

function nodeHeight(table: SchemaTable): number {
  return HEADER_HEIGHT + table.columns.length * ROW_HEIGHT;
}

function withSearch(nodes: TableNodeType[], search: string): TableNodeType[] {
  const term = search.trim().toLowerCase();
  if (!term) {
    return nodes.map((node) => ({ ...node, style: { ...node.style, opacity: 1 } }));
  }
  return nodes.map((node) => ({
    ...node,
    style: {
      ...node.style,
      opacity: node.data.table.name.toLowerCase().includes(term) ? 1 : 0.25,
    },
  }));
}

function TableNode({ data }: NodeProps<TableNodeType>) {
  const { table, onOpenTable } = data;

  return (
    <div
      className="w-[260px] overflow-hidden rounded-lg border border-line bg-panel shadow-xl transition-colors hover:border-accent/40"
      onDoubleClick={() => onOpenTable(table.name)}
      title="Double-click to browse rows"
    >
      <div className="flex items-center justify-between gap-2 border-b border-line bg-panel-3 px-3 py-2">
        <span className="truncate font-mono text-xs font-semibold text-fg">{table.name}</span>
        <span className="flex-none text-[9px] uppercase tracking-[0.08em] text-muted-2">
          {table.columns.length} cols
        </span>
      </div>

      <div className="divide-y divide-line/50">
        {table.columns.map((column) => (
          <div
            key={column.name}
            className="relative flex items-center gap-2 px-3 py-1.5"
          >
            <Handle
              type="target"
              position={Position.Left}
              id={`in-${column.name}`}
              className="!h-1.5 !w-1.5 !border-0 !bg-line-strong"
            />
            <span className="w-7 flex-none text-[8px] font-bold tracking-wide">
              {column.primary_key ? (
                <span className="text-accent">PK</span>
              ) : column.foreign_key ? (
                <span className="text-int">FK</span>
              ) : null}
            </span>
            <span className="truncate font-mono text-[11px] text-fg">{column.name}</span>
            <span className="ml-auto flex-none font-mono text-[8.5px] uppercase tracking-wide text-muted-2">
              {column.type_name}
            </span>
            <Handle
              type="source"
              position={Position.Right}
              id={`out-${column.name}`}
              className="!h-1.5 !w-1.5 !border-0 !bg-accent"
            />
          </div>
        ))}
      </div>
    </div>
  );
}

const nodeTypes = { table: TableNode };

function buildLayout(
  graph: SchemaGraph,
  onOpenTable: (table: string) => void,
): { nodes: TableNodeType[]; edges: Edge[] } {
  const layoutGraph = new dagre.graphlib.Graph();
  layoutGraph.setDefaultEdgeLabel(() => ({}));
  layoutGraph.setGraph({
    rankdir: "LR",
    ranksep: 120,
    nodesep: 60,
    marginx: 40,
    marginy: 40,
  });

  for (const table of graph.tables) {
    layoutGraph.setNode(table.name, {
      width: NODE_WIDTH,
      height: nodeHeight(table),
    });
  }

  const tableNames = new Set(graph.tables.map((table) => table.name));
  for (const relationship of graph.relationships) {
    if (
      relationship.from_table !== relationship.to_table &&
      tableNames.has(relationship.from_table) &&
      tableNames.has(relationship.to_table)
    ) {
      layoutGraph.setEdge(relationship.from_table, relationship.to_table);
    }
  }

  dagre.layout(layoutGraph);

  const nodes: TableNodeType[] = graph.tables.map((table) => {
    const position = layoutGraph.node(table.name);
    return {
      id: table.name,
      type: "table",
      position: {
        x: position.x - NODE_WIDTH / 2,
        y: position.y - nodeHeight(table) / 2,
      },
      data: { table, onOpenTable },
    };
  });

  const columnNames = new Map(
    graph.tables.map((table) => [table.name, new Set(table.columns.map((column) => column.name))]),
  );

  const edges: Edge[] = [];
  for (const relationship of graph.relationships) {
    if (
      !tableNames.has(relationship.from_table) ||
      !tableNames.has(relationship.to_table)
    ) {
      continue;
    }

    const sourceColumns = columnNames.get(relationship.from_table);
    const targetColumns = columnNames.get(relationship.to_table);
    const sourceColumn = sourceColumns?.has(relationship.from_column)
      ? relationship.from_column
      : sourceColumns?.values().next().value;
    const targetColumn = targetColumns?.has(relationship.to_column)
      ? relationship.to_column
      : targetColumns?.values().next().value;

    if (!sourceColumn || !targetColumn) continue;

    edges.push({
      id: relationship.id,
      source: relationship.from_table,
      target: relationship.to_table,
      sourceHandle: `out-${sourceColumn}`,
      targetHandle: `in-${targetColumn}`,
      type: "smoothstep",
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: "#2dd4bf",
        width: 14,
        height: 14,
      },
      style: { stroke: "#2dd4bf", strokeWidth: 1.5 },
      label: `${relationship.from_column} → ${relationship.to_column}`,
      labelStyle: {
        fill: "#7c8b9b",
        fontSize: 9,
        fontFamily: "monospace",
      },
      labelBgStyle: { fill: "#0f1317", fillOpacity: 0.92 },
      labelBgPadding: [4, 2],
      labelBgBorderRadius: 4,
    });
  }

  return { nodes, edges };
}

function loadPositions(key: string): StoredPositions {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return {};
    return parsed as StoredPositions;
  } catch {
    return {};
  }
}

function savePositions(key: string, nodes: TableNodeType[]) {
  try {
    const positions: StoredPositions = {};
    for (const node of nodes) {
      positions[node.id] = node.position;
    }
    localStorage.setItem(key, JSON.stringify(positions));
  } catch {
    // Positions are a convenience; ignore storage failures.
  }
}

export function DiagramView({
  graph,
  dbPath,
  onOpenTable,
  onCreateTable,
}: DiagramViewProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState<TableNodeType>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [flow, setFlow] = useState<ReactFlowInstance<TableNodeType, Edge> | null>(null);
  const [search, setSearch] = useState("");
  const onOpenTableRef = useRef(onOpenTable);
  onOpenTableRef.current = onOpenTable;
  const storageKey = `dbgo:diagram:${dbPath}`;

  const applyLayout = useCallback(() => {
    if (!graph) return;
    const layout = buildLayout(graph, (table) => onOpenTableRef.current(table));
    const nextNodes = withSearch(layout.nodes, search);
    setNodes(nextNodes);
    setEdges(layout.edges);
    savePositions(storageKey, nextNodes);
  }, [graph, search, storageKey, setEdges, setNodes]);

  useEffect(() => {
    if (!graph) {
      setNodes([]);
      setEdges([]);
      return;
    }

    const layout = buildLayout(graph, (table) => onOpenTableRef.current(table));
    const stored = loadPositions(storageKey);
    const nextNodes = withSearch(
      layout.nodes.map((node) =>
        stored[node.id] ? { ...node, position: stored[node.id] } : node,
      ),
      search,
    );

    setNodes(nextNodes);
    setEdges(layout.edges);

    const timer = window.setTimeout(() => {
      flow?.fitView({ padding: 0.2, duration: 300 });
    }, 60);
    return () => window.clearTimeout(timer);
    // Search changes are applied by the effect below without re-layout.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graph, storageKey, flow, setNodes, setEdges]);

  useEffect(() => {
    setNodes((current) => withSearch(current, search));
  }, [search, setNodes]);

  if (!graph || graph.tables.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2.5 p-10 text-center">
        <div className="grid h-[42px] w-[42px] place-items-center rounded-[11px] border border-line bg-panel-3 text-lg text-muted">
          ⌘
        </div>
        <h2 className="mt-1 text-base tracking-tight">No tables to diagram</h2>
        <p className="max-w-[420px] text-[13px] leading-relaxed text-muted">
          Create a table and its relationships will appear here.
        </p>
        <button
          type="button"
          className="inline-flex h-[38px] items-center justify-center rounded-[7px] border border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] px-5 text-[13px] font-bold text-[#03110f] hover:brightness-110"
          onClick={onCreateTable}
        >
          Create your first table
        </button>
      </div>
    );
  }

  return (
    <div className="relative flex min-h-0 flex-1 bg-ink">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        onInit={(instance) => setFlow(instance)}
        onNodeDragStop={(_event, _node, currentNodes) =>
          savePositions(storageKey, currentNodes)
        }
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.2}
        maxZoom={1.75}
        proOptions={{ hideAttribution: true }}
        className="bg-ink"
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          color="#1f2933"
        />
        <Controls />
        <MiniMap
          nodeColor="#1a222b"
          maskColor="rgba(10,12,15,0.72)"
          className="overflow-hidden rounded-lg border border-line"
        />
      </ReactFlow>

      <div className="pointer-events-none absolute inset-x-0 top-0 z-10 p-3">
        <div className="pointer-events-auto flex w-fit max-w-full flex-wrap items-center gap-2 rounded-lg border border-line bg-panel/90 p-2 shadow-lg backdrop-blur">
          <input
            className={INPUT}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search tables..."
            aria-label="Search diagram tables"
          />
          <button type="button" className={BUTTON} onClick={applyLayout}>
            Auto layout
          </button>
          <button
            type="button"
            className={BUTTON}
            onClick={() => flow?.fitView({ padding: 0.2, duration: 300 })}
          >
            Fit view
          </button>
          <button type="button" className={BUTTON} onClick={onCreateTable}>
            New table
          </button>
          <span className="px-1 font-mono text-[10px] text-muted-2">
            {graph.tables.length} tables · {graph.relationships.length} relations
          </span>
        </div>
      </div>
    </div>
  );
}
