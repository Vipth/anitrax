import * as React from "react";
import * as RadioGroup from "@radix-ui/react-radio-group";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogBody,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input, Textarea } from "@/components/ui/primitives";
import { cn } from "@/lib/utils";
import { MediaPoster } from "./MediaPoster";
import type { EntryPatch, ListStatus, MediaListEntry } from "@/lib/types";
import { STATUS_LABEL, STATUS_ORDER, mediaTitle } from "@/lib/format";
import { useEditEntry, useRemoveEntry } from "@/lib/hooks";

export function EditEntryDialog({
  entry,
  open,
  onOpenChange,
}: {
  entry: MediaListEntry;
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const edit = useEditEntry();
  const remove = useRemoveEntry();

  const [status, setStatus] = React.useState<ListStatus>(entry.status);
  const [progress, setProgress] = React.useState(String(entry.progress));
  const [scoreTen, setScoreTen] = React.useState(
    entry.scoreRaw > 0 ? String(entry.scoreRaw / 10) : "",
  );
  const [repeat, setRepeat] = React.useState(String(entry.repeat));
  const [notes, setNotes] = React.useState(entry.notes ?? "");
  const statusGroupRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    if (open) {
      setStatus(entry.status);
      setProgress(String(entry.progress));
      setScoreTen(entry.scoreRaw > 0 ? String(entry.scoreRaw / 10) : "");
      setRepeat(String(entry.repeat));
      setNotes(entry.notes ?? "");
    }
  }, [open, entry]);

  const save = () => {
    const patch: EntryPatch = {
      mediaId: entry.media.id.id,
      remoteId: entry.remoteId,
      status,
      progress: Math.max(0, parseInt(progress || "0", 10)),
      scoreRaw: scoreTen
        ? Math.round(Math.min(10, Math.max(0, parseFloat(scoreTen))) * 10)
        : 0,
      repeat: Math.max(0, parseInt(repeat || "0", 10)),
      notes: notes.trim() || null,
    };
    edit.mutate(patch, { onSuccess: () => onOpenChange(false) });
  };

  const del = () => {
    remove.mutate(entry.media.id.id, { onSuccess: () => onOpenChange(false) });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-md"
        onOpenAutoFocus={(e) => {
          // Land on the current status so arrow keys work immediately.
          e.preventDefault();
          statusGroupRef.current
            ?.querySelector<HTMLElement>('[role="radio"][data-state="checked"]')
            ?.focus();
        }}
      >
        <DialogHeader>
          <DialogTitle className="pr-8 leading-snug">
            {mediaTitle(entry.media)}
          </DialogTitle>
        </DialogHeader>

        <DialogBody>
          <div className="flex gap-4">
            <MediaPoster
              media={entry.media}
              className="h-32 w-24 shrink-0 rounded-md"
            />
            <div className="space-y-1 text-sm text-muted-foreground">
              {entry.media.title.native && <p>{entry.media.title.native}</p>}
              <p>
                {entry.media.episodes
                  ? `${entry.media.episodes} episodes`
                  : "Ongoing"}
              </p>
              {entry.media.averageScore != null && (
                <p>Community {entry.media.averageScore}%</p>
              )}
            </div>
          </div>

          <Field label="Status">
            <RadioGroup.Root
              ref={statusGroupRef}
              value={status}
              onValueChange={(v) => setStatus(v as ListStatus)}
              className="grid grid-cols-3 gap-1.5"
              aria-label="Status"
            >
              {STATUS_ORDER.map((s) => (
                <RadioGroup.Item
                  key={s}
                  value={s}
                  className={cn(
                    "rounded-md border px-2 py-1.5 text-xs font-medium outline-none transition-colors",
                    "focus-visible:ring-2 focus-visible:ring-ring/50",
                    "data-[state=checked]:border-primary data-[state=checked]:bg-primary/10 data-[state=checked]:text-primary",
                    "data-[state=unchecked]:border-border data-[state=unchecked]:text-muted-foreground data-[state=unchecked]:hover:bg-border/40",
                  )}
                >
                  {STATUS_LABEL[s]}
                </RadioGroup.Item>
              ))}
            </RadioGroup.Root>
          </Field>

          <div className="grid grid-cols-2 gap-4">
            <Field
              label={`Progress${entry.media.episodes ? ` / ${entry.media.episodes}` : ""}`}
            >
              <Input
                type="number"
                min={0}
                value={progress}
                onChange={(e) => setProgress(e.target.value)}
              />
            </Field>
            <Field label="Score (0–10)">
              <Input
                type="number"
                min={0}
                max={10}
                step={0.5}
                value={scoreTen}
                onChange={(e) => setScoreTen(e.target.value)}
                placeholder="–"
              />
            </Field>
          </div>

          <Field label="Rewatches">
            <Input
              type="number"
              min={0}
              value={repeat}
              onChange={(e) => setRepeat(e.target.value)}
            />
          </Field>

          <Field label="Notes">
            <Textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Private notes…"
            />
          </Field>

          <button
            onClick={del}
            disabled={remove.isPending}
            className="text-xs font-medium text-danger hover:underline disabled:opacity-50"
          >
            Remove from list
          </button>
        </DialogBody>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={save} disabled={edit.isPending}>
            {edit.isPending ? "Saving…" : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block space-y-1.5">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      {children}
    </label>
  );
}
