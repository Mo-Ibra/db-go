export function basename(path: string): string {
  return (
    path
      .split(/[\\/]/)
      .filter((part) => part && part !== ".")
      .pop() ?? path
  );
}
