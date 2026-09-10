import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { CircleAlert, Clock, Download, Pencil, Plus, Trash2 } from "lucide-react";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Card, Input } from "@/components/ui/primitives";
import { Switch } from "@/components/ui/switch";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  useCheckFeeds,
  useLibrary,
  useQbConfig,
  useRssFeeds,
  useRssHistory,
  useRssPollEnabled,
  useRssRules,
  useSetRssPollEnabled,
} from "@/lib/hooks";
import { mediaTitle, relativeTime } from "@/lib/format";
import { toast } from "@/stores/toast";
import { errorMessage } from "@/lib/types";
import type {
  MediaListEntry,
  MediaTitle,
  RssRule,
  RssRuleInput,
} from "@/lib/types";

export const Route = createFileRoute("/rss")({
  component: RssPage,
});

function titleOf(t: MediaTitle | null): string {
  return t?.english || t?.romaji || t?.native || "Unknown show";
}

const RES_OPTIONS = [
  { label: "Any", value: 0 },
  { label: "720p+", value: 720 },
  { label: "1080p+", value: 1080 },
  { label: "2160p+", value: 2160 },
];

function RssPage() {
  const rules = useRssRules();
  const history = useRssHistory();
  const qb = useQbConfig();
  const check = useCheckFeeds();
  const poll = useRssPollEnabled();
  const setPoll = useSetRssPollEnabled();
  const [editing, setEditing] = React.useState<RssRule | "new" | null>(null);

  const qbReady = !!qb.data?.baseUrl?.trim();

  const runCheck = () =>
    check.mutate(undefined, {
      onSuccess: (r) => {
        const msg = `${r.feedsChecked} feed${r.feedsChecked === 1 ? "" : "s"} · ${r.itemsSeen} items · ${r.added} added`;
        if (r.errors.length) toast.error("Checked with errors", `${msg}\n${r.errors[0]}`);
        else toast.success("Feeds checked", msg);
      },
      onError: (e) => toast.error("Check failed", errorMessage(e)),
    });

  return (
    <div className="mx-auto max-w-3xl px-6 py-6">
      <header className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold">RSS auto-download</h1>
          <p className="text-xs text-muted-foreground">
            Watch torrent feeds, match new episodes to a show you track, send
            them to qBittorrent. Progress stays manual.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-xs text-muted-foreground">
            <Switch
              checked={poll.data ?? true}
              onCheckedChange={(v) => setPoll.mutate(v)}
              aria-label="Check feeds automatically"
            />
            Auto every 15 min
          </label>
          <Button
            size="sm"
            variant="secondary"
            onClick={runCheck}
            disabled={check.isPending}
          >
            <Download
              className={cn("size-3.5", check.isPending && "animate-pulse")}
            />
            Check feeds now
          </Button>
        </div>
      </header>

      {!qb.isLoading && !qbReady && (
        <div className="mb-5 flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2.5 text-xs text-warning">
          <CircleAlert className="mt-0.5 size-4 shrink-0" />
          <span>
            qBittorrent isn&apos;t set up yet — rules will match but nothing gets
            downloaded.{" "}
            <Link to="/settings" className="font-medium underline">
              Connect it in Settings
            </Link>
            .
          </span>
        </div>
      )}

      <Section title="Feeds">
        <Feeds />
      </Section>

      <Section
        title="Rules"
        action={
          <Button size="sm" variant="ghost" onClick={() => setEditing("new")}>
            <Plus className="size-3.5" /> New rule
          </Button>
        }
      >
        {rules.isLoading ? (
          <p className="text-xs text-muted-foreground">Loading…</p>
        ) : (rules.data?.length ?? 0) === 0 ? (
          <p className="rounded-md border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
            No rules yet. A rule binds a feed to one of your tracked shows and
            says which releases to grab.
          </p>
        ) : (
          <ul className="divide-y divide-border rounded-md border border-border">
            {rules.data!.map((r) => (
              <RuleRow key={r.id} rule={r} onEdit={() => setEditing(r)} />
            ))}
          </ul>
        )}
      </Section>

      <Section title="Recent downloads">
        {history.isLoading ? (
          <p className="text-xs text-muted-foreground">Loading…</p>
        ) : (history.data?.length ?? 0) === 0 ? (
          <p className="rounded-md border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
            Nothing downloaded yet.
          </p>
        ) : (
          <ul className="divide-y divide-border rounded-md border border-border">
            {history.data!.map((h) => (
              <li key={h.guid} className="px-3 py-2">
                <p className="line-clamp-1 text-sm" title={h.title}>
                  {h.title}
                </p>
                <p className="mt-0.5 flex items-center gap-1.5 text-[11px] text-muted-foreground">
                  <Clock className="size-3" />
                  {relativeTime(h.downloadedAt)}
                  {h.ruleName ? ` · ${h.ruleName}` : ""}
                </p>
              </li>
            ))}
          </ul>
        )}
      </Section>

      {editing && (
        <RuleDialog
          rule={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  );
}

// --------------------------------------------------------------------------- //
// Feeds
// --------------------------------------------------------------------------- //

function Feeds() {
  const { data: feeds, isLoading } = useRssFeeds();
  const qc = useQueryClient();
  const [name, setName] = React.useState("");
  const [url, setUrl] = React.useState("");

  const invalidate = () => qc.invalidateQueries({ queryKey: qk.rssFeeds });

  const add = useMutation({
    mutationFn: () => api.addRssFeed(name.trim(), url.trim()),
    onSuccess: () => {
      setName("");
      setUrl("");
      invalidate();
      toast.success("Feed added");
    },
    onError: (e) => toast.error("Couldn't add feed", errorMessage(e)),
  });

  const remove = useMutation({
    mutationFn: (id: number) => api.removeRssFeed(id),
    onSuccess: invalidate,
  });

  const toggle = useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) =>
      api.setRssFeedEnabled(id, enabled),
    onSuccess: invalidate,
  });

  return (
    <div className="space-y-3">
      {isLoading ? (
        <p className="text-xs text-muted-foreground">Loading…</p>
      ) : feeds && feeds.length > 0 ? (
        <ul className="divide-y divide-border rounded-md border border-border">
          {feeds.map((f) => (
            <li key={f.id} className="flex items-center gap-3 px-3 py-2 text-sm">
              <Switch
                checked={f.enabled}
                onCheckedChange={(enabled) => toggle.mutate({ id: f.id, enabled })}
                aria-label={`Enable ${f.name}`}
              />
              <div className="min-w-0 flex-1">
                <p className="truncate" title={f.url}>
                  {f.name}
                </p>
                <p
                  className={cn(
                    "truncate text-[11px]",
                    f.lastError ? "text-danger" : "text-muted-foreground",
                  )}
                  title={f.lastError ?? undefined}
                >
                  {f.lastError
                    ? `Error: ${f.lastError}`
                    : f.lastFetchedAt
                      ? `Checked ${relativeTime(f.lastFetchedAt)}`
                      : "Not checked yet"}
                </p>
              </div>
              <button
                onClick={() => remove.mutate(f.id)}
                className="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-danger/10 hover:text-danger"
                aria-label="Remove feed"
              >
                <Trash2 className="size-3.5" />
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <p className="rounded-md border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
          No feeds yet. Add a torrent RSS feed URL (Nyaa, subsplease.org, …).
        </p>
      )}

      <form
        className="flex flex-wrap gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (url.trim()) add.mutate();
        }}
      >
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Name (optional)"
          className="w-40"
        />
        <Input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://nyaa.si/?page=rss&…"
          className="min-w-[200px] flex-1"
        />
        <Button size="sm" variant="secondary" type="submit" disabled={add.isPending || !url.trim()}>
          <Plus className="size-3.5" /> Add
        </Button>
      </form>
    </div>
  );
}

// --------------------------------------------------------------------------- //
// Rule row + dialog
// --------------------------------------------------------------------------- //

function ruleSummary(r: RssRule): string {
  const bits: string[] = [];
  if (r.titleContains) bits.push(`"${r.titleContains}"`);
  if (r.releaseGroup) bits.push(r.releaseGroup);
  if (r.minResolution) bits.push(`${r.minResolution}p+`);
  if (r.episodeFrom != null || r.episodeTo != null)
    bits.push(`ep ${r.episodeFrom ?? "…"}–${r.episodeTo ?? "…"}`);
  return bits.join(" · ") || "any release";
}

function RuleRow({ rule, onEdit }: { rule: RssRule; onEdit: () => void }) {
  const qc = useQueryClient();
  const invalidate = () => qc.invalidateQueries({ queryKey: qk.rssRules });

  const toggle = useMutation({
    mutationFn: (enabled: boolean) => api.setRssRuleEnabled(rule.id, enabled),
    onSuccess: invalidate,
  });
  const del = useMutation({
    mutationFn: () => api.deleteRssRule(rule.id),
    onSuccess: () => {
      invalidate();
      toast.info("Rule deleted");
    },
  });

  return (
    <li className="flex items-center gap-3 px-3 py-2.5 text-sm">
      <Switch
        checked={rule.enabled}
        onCheckedChange={(v) => toggle.mutate(v)}
        aria-label={`Enable ${rule.name}`}
      />
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium">
          {rule.name}
          {rule.paused && (
            <span className="ml-1.5 rounded bg-border/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
              adds paused
            </span>
          )}
        </p>
        <p className="truncate text-[11px] text-muted-foreground">
          {titleOf(rule.mediaTitle)} · {ruleSummary(rule)}
        </p>
      </div>
      <button
        onClick={onEdit}
        className="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-border/40 hover:text-foreground"
        aria-label="Edit rule"
      >
        <Pencil className="size-3.5" />
      </button>
      <button
        onClick={() => del.mutate()}
        className="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-danger/10 hover:text-danger"
        aria-label="Delete rule"
      >
        <Trash2 className="size-3.5" />
      </button>
    </li>
  );
}

function emptyInput(): RssRuleInput {
  return {
    name: "",
    enabled: true,
    feedId: null,
    service: null,
    mediaId: null,
    titleContains: null,
    releaseGroup: null,
    minResolution: null,
    episodeFrom: null,
    episodeTo: null,
    destPath: null,
    category: null,
    paused: false,
  };
}

function RuleDialog({
  rule,
  onClose,
}: {
  rule: RssRule | null;
  onClose: () => void;
}) {
  const qc = useQueryClient();
  const feeds = useRssFeeds();
  const library = useLibrary();
  const [form, setForm] = React.useState<RssRuleInput>(() =>
    rule
      ? {
          name: rule.name,
          enabled: rule.enabled,
          feedId: rule.feedId,
          service: rule.service,
          mediaId: rule.mediaId,
          titleContains: rule.titleContains,
          releaseGroup: rule.releaseGroup,
          minResolution: rule.minResolution,
          episodeFrom: rule.episodeFrom,
          episodeTo: rule.episodeTo,
          destPath: rule.destPath,
          category: rule.category,
          paused: rule.paused,
        }
      : emptyInput(),
  );
  const [showPick, setShowPick] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);

  const set = <K extends keyof RssRuleInput>(k: K, v: RssRuleInput[K]) =>
    setForm((f) => ({ ...f, [k]: v }));

  const pickShow = (entry: MediaListEntry) => {
    setForm((f) => ({
      ...f,
      mediaId: entry.media.id.id,
      service: entry.media.id.service,
      name: f.name.trim() || mediaTitle(entry.media),
      titleContains:
        f.titleContains?.trim() ||
        entry.media.title.romaji ||
        entry.media.title.english ||
        "",
      episodeFrom: f.episodeFrom ?? (entry.progress > 0 ? entry.progress + 1 : null),
    }));
    setShowPick("");
  };

  const currentShow = React.useMemo(() => {
    if (form.mediaId == null) return null;
    return library.data?.find((e) => e.media.id.id === form.mediaId) ?? null;
  }, [form.mediaId, library.data]);

  const matches = React.useMemo(() => {
    const q = showPick.trim().toLowerCase();
    if (!q) return [];
    return (library.data ?? [])
      .filter((e) => mediaTitle(e.media).toLowerCase().includes(q))
      .slice(0, 6);
  }, [showPick, library.data]);

  const save = useMutation({
    mutationFn: () => api.saveRssRule(form, rule?.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.rssRules });
      toast.success(rule ? "Rule updated" : "Rule created");
      onClose();
    },
    onError: (e) => toast.error("Couldn't save", errorMessage(e)),
  });

  const submit = () => {
    if (!form.name.trim()) return setError("Give the rule a name.");
    if (form.mediaId == null) return setError("Pick the show this rule is for.");
    setError(null);
    save.mutate();
  };

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{rule ? "Edit rule" : "New rule"}</DialogTitle>
        </DialogHeader>
        <DialogBody className="space-y-3">
          <Field label="Show">
            {currentShow ? (
              <div className="flex items-center justify-between gap-2 rounded-md border border-border px-3 py-2 text-sm">
                <span className="line-clamp-1">{mediaTitle(currentShow.media)}</span>
                <button
                  className="text-xs text-muted-foreground hover:text-foreground"
                  onClick={() => {
                    set("mediaId", null);
                    set("service", null);
                  }}
                >
                  Change
                </button>
              </div>
            ) : (
              <>
                <Input
                  value={showPick}
                  onChange={(e) => setShowPick(e.target.value)}
                  placeholder="Filter your tracked shows…"
                />
                {matches.length > 0 && (
                  <ul className="mt-1 divide-y divide-border rounded-md border border-border">
                    {matches.map((e) => (
                      <li key={e.media.id.id}>
                        <button
                          className="w-full px-3 py-1.5 text-left text-sm hover:bg-border/40"
                          onClick={() => pickShow(e)}
                        >
                          {mediaTitle(e.media)}
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </>
            )}
          </Field>

          <Field label="Rule name">
            <Input
              value={form.name}
              onChange={(e) => set("name", e.target.value)}
              placeholder="e.g. Frieren 1080p"
            />
          </Field>

          <div className="grid grid-cols-2 gap-3">
            <Field label="Feed">
              <select
                value={form.feedId ?? ""}
                onChange={(e) =>
                  set("feedId", e.target.value ? Number(e.target.value) : null)
                }
                className="h-9 w-full rounded-md border border-border bg-surface px-2 text-sm outline-none focus:border-primary"
              >
                <option value="">All feeds</option>
                {(feeds.data ?? []).map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.name}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Min resolution">
              <select
                value={form.minResolution ?? 0}
                onChange={(e) =>
                  set("minResolution", Number(e.target.value) || null)
                }
                className="h-9 w-full rounded-md border border-border bg-surface px-2 text-sm outline-none focus:border-primary"
              >
                {RES_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </Field>
          </div>

          <Field
            label="Title must contain"
            hint="Case-insensitive, every word must appear in the release title."
          >
            <Input
              value={form.titleContains ?? ""}
              onChange={(e) => set("titleContains", e.target.value || null)}
              placeholder="Frieren"
            />
          </Field>

          <div className="grid grid-cols-3 gap-3">
            <Field label="Release group">
              <Input
                value={form.releaseGroup ?? ""}
                onChange={(e) => set("releaseGroup", e.target.value || null)}
                placeholder="SubsPlease"
              />
            </Field>
            <Field label="Episode from">
              <Input
                type="number"
                min={0}
                value={form.episodeFrom ?? ""}
                onChange={(e) =>
                  set("episodeFrom", e.target.value ? Number(e.target.value) : null)
                }
              />
            </Field>
            <Field label="Episode to">
              <Input
                type="number"
                min={0}
                value={form.episodeTo ?? ""}
                onChange={(e) =>
                  set("episodeTo", e.target.value ? Number(e.target.value) : null)
                }
              />
            </Field>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <Field label="Save path" hint="Blank = qBittorrent default.">
              <Input
                value={form.destPath ?? ""}
                onChange={(e) => set("destPath", e.target.value || null)}
                placeholder="D:\Anime\Frieren"
              />
            </Field>
            <Field label="qBittorrent category">
              <Input
                value={form.category ?? ""}
                onChange={(e) => set("category", e.target.value || null)}
                placeholder="anime"
              />
            </Field>
          </div>

          <div className="flex items-center gap-6 pt-1">
            <label className="flex items-center gap-2 text-xs text-muted-foreground">
              <Switch
                checked={form.enabled}
                onCheckedChange={(v) => set("enabled", v)}
              />
              Enabled
            </label>
            <label className="flex items-center gap-2 text-xs text-muted-foreground">
              <Switch
                checked={form.paused}
                onCheckedChange={(v) => set("paused", v)}
              />
              Add torrents paused
            </label>
          </div>

          {error && <p className="text-xs text-danger">{error}</p>}
        </DialogBody>
        <DialogFooter>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" onClick={submit} disabled={save.isPending}>
            {rule ? "Save changes" : "Create rule"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// --------------------------------------------------------------------------- //

function Section({
  title,
  action,
  children,
}: {
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="mb-6">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {title}
        </h2>
        {action}
      </div>
      <Card className="p-4">{children}</Card>
    </section>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-xs font-medium">{label}</span>
      {children}
      {hint && <span className="block text-[11px] text-muted-foreground">{hint}</span>}
    </label>
  );
}
