import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ExternalLink,
  FolderPlus,
  LogOut,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { Button } from "@/components/ui/button";
import { Card, Input } from "@/components/ui/primitives";
import { Switch } from "@/components/ui/switch";
import { ThemeSelect } from "@/components/layout/ThemeSelect";
import { RequestBudgetMeter } from "@/components/RequestBudgetMeter";
import { useLibraryFolders, useScanLibrary, useSettings } from "@/lib/hooks";
import { relativeTime } from "@/lib/format";
import { toast } from "@/stores/toast";
import { errorMessage, type AppSettings } from "@/lib/types";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsPage() {
  const { data: settings } = useSettings();
  const qc = useQueryClient();
  const account = settings?.accounts.find((a) => a.service === "anilist");

  return (
    <div className="mx-auto max-w-2xl px-6 py-6">
      <h1 className="mb-6 text-lg font-semibold">Settings</h1>

      <Section title="AniList account">
        {account ? (
          <div className="flex items-center gap-3">
            {account.avatarUrl && (
              <img
                src={account.avatarUrl}
                alt=""
                className="size-10 rounded-full"
              />
            )}
            <div className="flex-1">
              <p className="text-sm font-medium">{account.userName}</p>
              <p className="text-xs text-muted-foreground">
                Connected · score format {account.scoreFormat}
              </p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={async () => {
                await api.disconnect("anilist");
                qc.invalidateQueries({ queryKey: qk.settings });
                qc.invalidateQueries({ queryKey: qk.library() });
                toast.info("Disconnected");
              }}
            >
              <LogOut className="size-3.5" /> Disconnect
            </Button>
          </div>
        ) : (
          <ConnectFlow clientId={settings?.anilistClientId ?? ""} redirect={settings?.anilistRedirect ?? ""} />
        )}
      </Section>

      <Section title="Sync">
        <SettingRow
          label="Sync on startup"
          hint="Pull your list from AniList every time the app opens. Off = launch straight from the local cache; use the Sync button to refresh."
        >
          <SyncOnStartupToggle />
        </SettingRow>
      </Section>

      <Section title="Watched folders">
        <WatchedFolders />
      </Section>

      <Section title="Appearance">
        <SettingRow label="Theme">
          <ThemeSelect />
        </SettingRow>
      </Section>

      <Section title="API usage">
        <RequestBudgetMeter />
        <p className="mt-3 text-xs leading-relaxed text-muted-foreground">
          Every AniList call — list sync, search, detail views, episode edits —
          goes through one paced queue capped at 45 requests/minute, half of
          AniList&apos;s limit. Your list is served from a local cache, so
          scrolling and reopening screens never hits the network.
        </p>
      </Section>
    </div>
  );
}

const PIN_REDIRECT = "https://anilist.co/api/v2/oauth/pin";

function ConnectFlow({
  clientId,
  redirect,
}: {
  clientId: string;
  redirect: string;
}) {
  const qc = useQueryClient();
  const [id, setId] = React.useState(clientId);
  const [token, setToken] = React.useState("");

  React.useEffect(() => setId(clientId), [clientId]);

  const saveId = useMutation({
    mutationFn: (v: string) => api.setAnilistClientId(v),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.settings });
      toast.success("Saved client ID");
    },
  });

  const startLogin = useMutation({
    mutationFn: async () => {
      const url = await api.anilistLoginUrl();
      await openUrl(url);
    },
    onError: (e) => toast.error("Can't start sign-in", errorMessage(e)),
  });

  const connectToken = useMutation({
    mutationFn: (t: string) => api.anilistConnectToken(t),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.settings });
      qc.invalidateQueries({ queryKey: qk.library() });
      toast.success("Connected", "Your list is syncing.");
    },
    onError: (e) => toast.error("Couldn't connect", errorMessage(e)),
  });

  return (
    <div className="space-y-4">
      <ol className="space-y-4 text-sm">
        <li className="space-y-1.5">
          <p className="font-medium">1. Create an API client</p>
          <p className="text-xs leading-relaxed text-muted-foreground">
            Open{" "}
            <button
              className="inline-flex items-center gap-1 text-primary hover:underline"
              onClick={() => openUrl("https://anilist.co/settings/developer")}
            >
              anilist.co/settings/developer <ExternalLink className="size-3" />
            </button>{" "}
            → “Create New Client”. Set its <strong>Redirect URL</strong> to one of
            these:
          </p>
          <div className="space-y-1.5">
            <RedirectOption
              url={PIN_REDIRECT}
              label="Easiest — AniList shows you a token to paste below"
            />
            <RedirectOption
              url={redirect || "anitrax://oauth/anilist"}
              label="Seamless — the app captures sign-in automatically"
            />
          </div>
        </li>

        <li className="space-y-1.5">
          <p className="font-medium">2. Paste the client ID</p>
          <div className="flex gap-2">
            <Input
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder="e.g. 12345"
            />
            <Button
              size="sm"
              variant="secondary"
              disabled={!id.trim() || saveId.isPending}
              onClick={() => saveId.mutate(id.trim())}
            >
              Save
            </Button>
          </div>
        </li>

        <li className="space-y-1.5">
          <p className="font-medium">3. Authorize</p>
          <Button
            size="sm"
            disabled={!clientId || startLogin.isPending}
            onClick={() => startLogin.mutate()}
          >
            Sign in with AniList
          </Button>
          <p className="text-xs text-muted-foreground">
            {!clientId
              ? "Save your client ID first."
              : "Approve in your browser. With the custom-scheme redirect the app connects itself; with the pin redirect, copy the token AniList shows you."}
          </p>
        </li>

        <li className="space-y-1.5">
          <p className="font-medium">4. Paste the token (pin redirect only)</p>
          <div className="flex gap-2">
            <Input
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Access token from AniList"
              type="password"
            />
            <Button
              size="sm"
              disabled={!token.trim() || connectToken.isPending}
              onClick={() => connectToken.mutate(token.trim())}
            >
              Connect
            </Button>
          </div>
        </li>
      </ol>
    </div>
  );
}

function RedirectOption({ url, label }: { url: string; label: string }) {
  const [copied, setCopied] = React.useState(false);
  return (
    <button
      onClick={async () => {
        await navigator.clipboard.writeText(url).catch(() => {});
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="block w-full rounded-md border border-border bg-surface px-2 py-1.5 text-left transition-colors hover:border-primary"
    >
      <code className="text-xs">{url}</code>
      <span className="mt-0.5 block text-[11px] text-muted-foreground">
        {copied ? "Copied!" : label}
      </span>
    </button>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mb-6">
      <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </h2>
      <Card className="p-4">{children}</Card>
    </section>
  );
}

function SettingRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm">{label}</p>
        {hint && (
          <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
            {hint}
          </p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function WatchedFolders() {
  const { data: folders, isLoading } = useLibraryFolders();
  const qc = useQueryClient();
  const scan = useScanLibrary();

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: qk.libraryFolders });
    qc.invalidateQueries({ queryKey: qk.libraryFiles });
    qc.invalidateQueries({ queryKey: qk.libraryOwned });
  };

  const add = useMutation({
    mutationFn: async () => {
      const picked = await open({ directory: true, multiple: false });
      if (typeof picked !== "string") return null;
      return api.addLibraryFolder(picked);
    },
    onSuccess: (f) => {
      if (!f) return;
      invalidate();
      toast.success("Folder added", `Scanning ${f.path}`);
    },
    onError: (e) => toast.error("Couldn't add folder", errorMessage(e)),
  });

  const remove = useMutation({
    mutationFn: (id: number) => api.removeLibraryFolder(id),
    onSuccess: invalidate,
    onError: (e) => toast.error("Couldn't remove folder", errorMessage(e)),
  });

  const toggle = useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) =>
      api.setLibraryFolderEnabled(id, enabled),
    onSuccess: invalidate,
    onError: (e) => toast.error("Couldn't update folder", errorMessage(e)),
  });

  const runScan = () =>
    scan.mutate(undefined, {
      onSuccess: (r) =>
        toast.success(
          "Scan complete",
          `${r.filesSeen} files · ${r.autoMatched} matched · ${r.unmatched} to review`,
        ),
      onError: (e) => toast.error("Scan failed", errorMessage(e)),
    });

  return (
    <div className="space-y-3">
      <p className="text-xs leading-relaxed text-muted-foreground">
        AniTrax scans these folders for episode files and matches them to your
        list — entirely offline. Matching uses titles already in your cache, so a
        show you&apos;ve never synced or searched won&apos;t match until you link
        it by hand.
      </p>

      {isLoading ? (
        <p className="text-xs text-muted-foreground">Loading…</p>
      ) : folders && folders.length > 0 ? (
        <ul className="divide-y divide-border rounded-md border border-border">
          {folders.map((f) => (
            <li
              key={f.id}
              className="flex items-center gap-3 px-3 py-2 text-sm"
            >
              <Switch
                checked={f.enabled}
                onCheckedChange={(enabled) =>
                  toggle.mutate({ id: f.id, enabled })
                }
                aria-label={`Watch ${f.path}`}
              />
              <div className="min-w-0 flex-1">
                <p className="truncate" title={f.path}>
                  {f.path}
                </p>
                <p className="text-[11px] text-muted-foreground">
                  {f.fileCount} file{f.fileCount === 1 ? "" : "s"}
                  {f.scannedAt
                    ? ` · scanned ${relativeTime(f.scannedAt)}`
                    : " · not scanned yet"}
                </p>
              </div>
              <button
                onClick={() => remove.mutate(f.id)}
                className="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-danger/10 hover:text-danger"
                aria-label="Remove folder"
              >
                <Trash2 className="size-3.5" />
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <p className="rounded-md border border-dashed border-border px-3 py-4 text-center text-xs text-muted-foreground">
          No folders yet. Add the folder where your episodes live.
        </p>
      )}

      <div className="flex gap-2">
        <Button
          size="sm"
          variant="secondary"
          onClick={() => add.mutate()}
          disabled={add.isPending}
        >
          <FolderPlus className="size-3.5" /> Add folder
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={runScan}
          disabled={scan.isPending || !folders?.length}
        >
          <RefreshCw
            className={scan.isPending ? "size-3.5 animate-spin" : "size-3.5"}
          />
          Rescan now
        </Button>
      </div>
    </div>
  );
}

function SyncOnStartupToggle() {
  const { data: settings } = useSettings();
  const qc = useQueryClient();
  const enabled = settings?.syncOnStartup ?? true;

  const mut = useMutation({
    mutationFn: (v: boolean) => api.setSyncOnStartup(v),
    onMutate: async (v) => {
      await qc.cancelQueries({ queryKey: qk.settings });
      const prev = qc.getQueryData<AppSettings>(qk.settings);
      if (prev) qc.setQueryData<AppSettings>(qk.settings, { ...prev, syncOnStartup: v });
      return { prev };
    },
    onError: (e, _v, ctx) => {
      if (ctx?.prev) qc.setQueryData(qk.settings, ctx.prev);
      toast.error("Couldn't save", errorMessage(e));
    },
    onSettled: () => qc.invalidateQueries({ queryKey: qk.settings }),
  });

  return (
    <Switch
      checked={enabled}
      onCheckedChange={(v) => mut.mutate(v)}
      aria-label="Sync on startup"
    />
  );
}
