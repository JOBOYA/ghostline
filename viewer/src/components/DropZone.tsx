import { useCallback, useState, useRef } from 'react';
import { useStore } from '../store/useStore';
import { parseGhostline } from '../lib/parser';

export function DropZone() {
  const loadRun = useStore((s) => s.loadRun);
  const [dragging, setDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFiles = useCallback(
    async (files: FileList | null) => {
      if (!files) return;
      for (const file of Array.from(files)) {
        if (!file.name.endsWith('.ghostline')) continue;
        try {
          const buf = await file.arrayBuffer();
          const run = await parseGhostline(buf, file.name);
          loadRun(run);
        } catch (err) {
          console.error('Failed to parse', file.name, err);
        }
      }
    },
    [loadRun],
  );

  return (
    <div
      onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        handleFiles(e.dataTransfer.files);
      }}
      onClick={() => inputRef.current?.click()}
      style={{
        margin: 8,
        padding: 16,
        border: `2px dashed ${dragging ? 'var(--accent-llm)' : 'var(--border)'}`,
        borderRadius: 8,
        textAlign: 'center',
        color: 'var(--muted)',
        fontSize: 12,
        cursor: 'pointer',
        transition: 'border-color 0.15s',
      }}
    >
      <input
        ref={inputRef}
        type="file"
        accept=".ghostline"
        multiple
        style={{ display: 'none' }}
        onChange={(e) => handleFiles(e.target.files)}
      />
      + Drop or click
    </div>
  );
}
