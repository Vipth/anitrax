import * as React from "react";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetBody,
  SheetFooter,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Input, Textarea } from "@/components/ui/primitives";
import { MediaPoster } from "./MediaPoster";
import type { EntryPatch, ListStatus, MediaListEntry } from "@/lib/types";
import { STATUS_LABEL, STATUS_ORDER, mediaTitle } from "@/lib/format";
import { useEditEntry, useRemoveEntry } from "@/lib/hooks";

export function EditEntrySheet({
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
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle className="pr-8">{mediaTitle(entry.media)}</SheetTitle>
        </SheetHeader>
        <SheetBody>
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
            <div className="grid grid-cols-3 gap-1.5">
              {STATUS_ORDER.map((s) => (
                <button
                  key={s}
                  onClick={() => setStatus(s)}
                  className={
                    "rounded-md border px-2 py-1.5 text-xs font-medium transition-colors " +
                    (status === s
                      ? "border-primary bg-primary/10 text-primary"
                      : "border-border text-muted-foreground hover:bg-border/40")
                  }
                >
                  {STATUS_LABEL[s]}
                </button>
              ))}
            </div>
          </Field>

          <div className="grid grid-cols-2 gap-4">
            <Field label={`Progress${entry.media.episodes ? ` / ${entry.media.episodes}` : ""}`}>
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
            className="text-xs font-medium text-danger hover:underline"
          >
            Remove from list
          </button>
        </SheetBody>
        <SheetFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={save} disabled={edit.isPending}>
            {edit.isPending ? "Saving…" : "Save"}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
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
