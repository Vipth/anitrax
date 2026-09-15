import type * as React from "react";
import { useNavigate } from "@tanstack/react-router";
import { ExternalLink, FolderOpen, Pencil, Play, Rss, Trash2 } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useRemoveEntry } from "@/lib/hooks";
import { nextEpisodeOnDisk } from "@/lib/library";
import { api } from "@/lib/ipc";
import { useUi } from "@/stores/ui";
import { toast } from "@/stores/toast";
import { errorMessage, type MediaListEntry } from "@/lib/types";

/** Right-click menu for a library entry — the same actions available
 * elsewhere (edit dialog, play button, RSS rule builder, remove) in one place. */
export function EntryContextMenu({
  entry,
  owned,
  onEdit,
  children,
}: {
  entry: MediaListEntry;
  owned?: number[];
  onEdit: (entry: MediaListEntry) => void;
  children: React.ReactNode;
}) {
  const navigate = useNavigate();
  const remove = useRemoveEntry();
  const playEp = nextEpisodeOnDisk(entry, owned);

  const play = async () => {
    if (playEp == null) return;
    try {
      await api.playEpisode(entry.media.id.id, playEp, entry.media.id.service);
    } catch (err) {
      toast.error("Couldn't play", errorMessage(err));
    }
  };

  const makeRssRule = () => {
    useUi.getState().setRssRulePrefillMediaId(entry.media.id.id);
    navigate({ to: "/rss" });
  };

  const openFolder = async () => {
    try {
      await api.openMediaFolder(entry.media.id.id);
    } catch (err) {
      toast.error("Couldn't open folder", errorMessage(err));
    }
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onSelect={() => onEdit(entry)}>
          <Pencil className="size-3.5" /> Edit
        </ContextMenuItem>
        {playEp != null && (
          <ContextMenuItem onSelect={play}>
            <Play className="size-3.5" /> Play episode {playEp}
          </ContextMenuItem>
        )}
        {owned && owned.length > 0 && (
          <ContextMenuItem onSelect={openFolder}>
            <FolderOpen className="size-3.5" /> Open local files
          </ContextMenuItem>
        )}
        <ContextMenuItem onSelect={makeRssRule}>
          <Rss className="size-3.5" /> Make RSS rule
        </ContextMenuItem>
        {entry.media.id.service === "anilist" && (
          <ContextMenuItem
            onSelect={() => openUrl(`https://anilist.co/anime/${entry.media.id.id}`)}
          >
            <ExternalLink className="size-3.5" /> View on AniList
          </ContextMenuItem>
        )}
        <ContextMenuSeparator />
        <ContextMenuItem danger onSelect={() => remove.mutate(entry.media.id.id)}>
          <Trash2 className="size-3.5" /> Remove from list
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
