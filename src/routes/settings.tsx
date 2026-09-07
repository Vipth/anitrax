import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ExternalLink, LogOut } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { Button } from "@/components/ui/button";
import { Card, Input } from "@/components/ui/primitives";
import { ThemeToggle } from "@/components/layout/ThemeToggle";
import { RequestBudgetMeter } from "@/components/RequestBudgetMeter";
import { useSettings } from "@/lib/hooks";
import { toast } from "@/stores/toast";
import { errorMessage } from "@/lib/types";

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

      <Section title="Appearance">
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">Theme</span>
          <ThemeToggle />
        </div>
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
  const [showManual, setShowManual] = React.useState(false);

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
      <ol className="space-y-3 text-sm">
        <li className="space-y-1.5">
          <p className="font-medium">1. Register an API client</p>
          <p className="text-xs text-muted-foreground">
            Open{" "}
            <button
              className="inline-flex items-center gap-1 text-primary hover:underline"
              onClick={() => openUrl("https://anilist.co/settings/developer")}
            >
              anilist.co/settings/developer <ExternalLink className="size-3" />
            </button>{" "}
            → “Create New Client”. Set the redirect URL to:
          </p>
          <code className="block rounded-md bg-surface px-2 py-1.5 text-xs">
            {redirect}
          </code>
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
          <p className="font-medium">3. Sign in</p>
          <Button
            size="sm"
            disabled={!clientId || startLogin.isPending}
            onClick={() => startLogin.mutate()}
          >
            Sign in with AniList
          </Button>
          {!clientId && (
            <p className="text-xs text-muted-foreground">
              Save your client ID first.
            </p>
          )}
        </li>
      </ol>

      <button
        className="text-xs text-muted-foreground underline"
        onClick={() => setShowManual((v) => !v)}
      >
        {showManual ? "Hide" : "Sign-in didn't redirect back?"}
      </button>

      {showManual && (
        <div className="space-y-2 rounded-md border border-border bg-surface p-3">
          <p className="text-xs text-muted-foreground">
            On the developer page you can also generate an access token directly
            (“Auth Token”). Paste it here:
          </p>
          <div className="flex gap-2">
            <Input
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Access token"
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
        </div>
      )}
    </div>
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
