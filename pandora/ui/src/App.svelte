<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import EqDrawer from "./components/EqDrawer.svelte";
  import HomeView from "./components/HomeView.svelte";
  import IceHubView from "./components/IceHubView.svelte";
  import Icon from "./components/Icon.svelte";
  import LibraryView from "./components/LibraryView.svelte";
  import NowPlayingOverlay from "./components/NowPlayingOverlay.svelte";
  import PlayerBar from "./components/PlayerBar.svelte";
  import PlaylistModal from "./components/PlaylistModal.svelte";
  import RecognizeView from "./components/RecognizeView.svelte";
  import SearchView from "./components/SearchView.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import Sidebar, { type NavTab } from "./components/Sidebar.svelte";
  import { audioEngine, playerStore } from "./lib/audio";
  import {
    addLocalFolder,
    addToPlaylist,
    clearOfflineCache,
    createPlaylist,
    deleteCachedTrack,
    deletePlaylist,
    downloadTrackOffline,
    getAppPaths,
    getIceSnapshot,
    getIceStatus,
    getLibrary,
    getProxyPort,
    isDemoMode,
    isTauriEnv,
    onIceMeter,
    onIceSnapshot,
    onIceStatus,
    removeFromPlaylist,
    removeLocalFolder,
    renamePlaylist,
    rescanLocalFolders,
    resetPandoraSettings,
    savePandoraSettings,
    setIceModePref,
    toggleLikeTrack,
    ytmHome,
  } from "./lib/ipc";
  import {
    DEFAULT_PANDORA_SETTINGS,
    type AppPaths,
    type EngineSnapshot,
    type HomeFeed,
    type IceStatus,
    type LibraryState,
    type MeterFrame,
    type PandoraSettings,
    type Track,
  } from "./lib/types";

  let activeTab = $state<NavTab>("home");
  let libraryInitialTab = $state<
    "liked" | "playlists" | "local" | "offline" | "history"
  >("liked");
  let selectedPlaylistId = $state<string | null>(null);
  let showNowPlaying = $state(false);
  let showEq = $state(false);
  let playlistModalOpen = $state(false);
  let playlistModalTrack = $state<Track | null>(null);
  let statusNotice = $state<string | null>(null);
  let noticeTimer: number | null = null;

  let proxyPort = $state(0);
  let demoMode = $state(false);
  let feedLoading = $state(false);
  let externalQuery = $state("");
  let externalQuerySeq = $state(0);
  let feed = $state<HomeFeed>({
    quickPicks: [],
    trending: [],
    offlineFallback: false,
    error: null,
  });
  let library = $state<LibraryState>({
    liked: [],
    playlists: [],
    history: [],
    recognitionHistory: [],
    localFolders: [],
    localTracks: [],
    tracksIndex: {},
    cachedIds: [],
    cacheBytes: 0,
    eq: {
      enabled: true,
      preset: "Flat",
      bands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      normalize: false,
      skipSilence: false,
      playbackRate: 1,
    },
    iceModePref: "auto",
    settings: { ...DEFAULT_PANDORA_SETTINGS },
  });

  let appPaths = $state<AppPaths | null>(null);
  let iceStatus = $state<IceStatus>({
    mode: "unavailable",
    standaloneRunning: false,
    standaloneExe: null,
    configPath: "",
    bridgeConnected: false,
    bridgePort: null,
    bridgePid: null,
    notchExpanded: null,
    iceVersion: null,
    bridgeInfoPath: "",
    error: null,
  });
  let iceSnapshot = $state<EngineSnapshot | null>(null);
  let iceMeter = $state<MeterFrame | null>(null);

  let unlistenStatus: (() => void) | null = null;
  let unlistenSnap: (() => void) | null = null;
  let unlistenMeter: (() => void) | null = null;

  let likedIds = $derived(new Set(library.liked.map((t) => t.id)));
  $effect(() => {
    document.documentElement.dataset.theme = library.settings.theme;
  });
  let cachedIds = $derived(new Set(library.cachedIds));
  let defaultDevice = $derived(iceSnapshot?.playback.find((d) => d.isDefault));

  function showNotice(msg: string) {
    statusNotice = msg;
    if (noticeTimer !== null) window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => {
      statusNotice = null;
      noticeTimer = null;
    }, 2400);
  }

  onMount(async () => {
    proxyPort = await getProxyPort().catch(() => 0);
    demoMode = await isDemoMode().catch(() => false);
    const lib = await getLibrary().catch(() => library);
    library = lib;
    audioEngine.init(proxyPort, lib.eq, lib.settings, () => {
      void getLibrary()
        .then((next) => (library = next))
        .catch(() => {});
    });

    if (isTauriEnv()) {
      unlistenStatus = await onIceStatus((st) => (iceStatus = st));
      unlistenSnap = await onIceSnapshot((sn) => (iceSnapshot = sn));
      unlistenMeter = await onIceMeter((fr) => (iceMeter = fr));
      void getIceStatus()
        .then((st) => (iceStatus = st))
        .catch(() => {});
      void getIceSnapshot()
        .then((sn) => (iceSnapshot = sn))
        .catch(() => {});
      void getAppPaths().then((p) => (appPaths = p)).catch(() => {});
    }

    await refreshHome();
  });

  onDestroy(() => {
    if (noticeTimer !== null) window.clearTimeout(noticeTimer);
    unlistenStatus?.();
    unlistenSnap?.();
    unlistenMeter?.();
  });

  async function refreshHome() {
    feedLoading = true;
    try {
      feed = await ytmHome();
    } finally {
      feedLoading = false;
    }
  }

  function navigate(tab: NavTab, playlistId: string | null = null) {
    activeTab = tab;
    selectedPlaylistId = playlistId;
  }

  function openPlaylistModal(track: Track | null) {
    playlistModalTrack = track;
    playlistModalOpen = true;
  }

  async function handleToggleLike(track: Track) {
    try {
      const next = await toggleLikeTrack(track);
      library = next;
      const nowLiked = next.liked.some((t) => t.id === track.id);
      if ($playerStore.currentTrack?.id === track.id) {
        playerStore.update((s) =>
          s.currentTrack
            ? { ...s, currentTrack: { ...s.currentTrack, liked: nowLiked } }
            : s,
        );
      }
      showNotice(
        nowLiked
          ? `Added "${track.title}" to Favorites`
          : `Removed "${track.title}" from Favorites`,
      );
    } catch (e) {
      playerStore.update((s) => ({
        ...s,
        error: e instanceof Error ? e.message : String(e),
      }));
    }
  }

  async function handleCreatePlaylist(
    name: string,
    initialTrack: Track | null = null,
  ) {
    const next = await createPlaylist(name, initialTrack);
    library = next;
    const created = next.playlists[next.playlists.length - 1];
    if (created && !initialTrack) {
      navigate("library", created.id);
    }
    showNotice(
      initialTrack
        ? `Created "${name}" and added "${initialTrack.title}"`
        : `Created playlist "${name}"`,
    );
  }

  async function handleToggleTrackInPlaylist(
    playlistId: string,
    track: Track,
    inPlaylist: boolean,
  ) {
    if (inPlaylist) {
      library = await removeFromPlaylist(playlistId, track.id);
      showNotice(`Removed "${track.title}" from playlist`);
    } else {
      library = await addToPlaylist(playlistId, track);
      showNotice(`Added "${track.title}" to playlist`);
    }
  }

  function playShuffled(list: Track[]) {
    if (list.length === 0) return;
    const shuffled = [...list].sort(() => Math.random() - 0.5);
    void audioEngine.playTrack(shuffled[0], shuffled);
  }

  async function handleUpdateSettings(patch: Partial<PandoraSettings>) {
    const merged: PandoraSettings = { ...library.settings, ...patch };
    library = { ...library, settings: merged };
    audioEngine.applySettings(merged);
    if (isTauriEnv()) {
      try {
        library = await savePandoraSettings(merged);
      } catch (e) {
        showNotice(`Could not save setting: ${e instanceof Error ? e.message : String(e)}`);
      }
    }
  }

  async function handleResetSettings() {
    if (isTauriEnv()) {
      library = await resetPandoraSettings();
    } else {
      library = { ...library, settings: { ...DEFAULT_PANDORA_SETTINGS } };
    }
    audioEngine.applySettings(library.settings);
    audioEngine.updateEq(library.eq);
    showNotice("Pandora settings and EQ reset to defaults");
  }

  async function handleSetModePref(pref: "auto" | "embedded" | "connected") {
    library = { ...library, iceModePref: pref };
    if (isTauriEnv()) {
      try {
        iceStatus = await setIceModePref(pref);
        iceSnapshot = await getIceSnapshot();
        showNotice(`Ice engine preference set to ${pref}`);
      } catch (e) {
        showNotice(e instanceof Error ? e.message : String(e));
      }
    }
  }
</script>

<div class="ambient-bg" aria-hidden="true"></div>

<div class="shell">
  <div class="main-area">
    <Sidebar
      {activeTab}
      {selectedPlaylistId}
      playlists={library.playlists}
      {iceStatus}
      {defaultDevice}
      showIceCard={library.settings.showIceCardInSidebar}
      onNavigate={navigate}
      onOpenNewPlaylist={() => openPlaylistModal(null)}
    />

    <main class="content">
      {#if $playerStore.error}
        <div class="toast-err">
          <span>{$playerStore.error}</span>
          <button
            aria-label="Dismiss error"
            onclick={() => playerStore.update((s) => ({ ...s, error: null }))}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
      {/if}

      {#if statusNotice}
        <div class="toast-ok">{statusNotice}</div>
      {/if}

      {#if activeTab === "home"}
        <HomeView
          {feed}
          loading={feedLoading}
          {demoMode}
          history={library.history}
          favorites={library.liked}
          {likedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          isPlaying={$playerStore.playing}
          {proxyPort}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onToggleLike={(track) => void handleToggleLike(track)}
          onOpenPlaylistPicker={(track) => openPlaylistModal(track)}
          onRefresh={() => void refreshHome()}
          onGoSearch={() => navigate("search")}
          onOpenLocal={() => {
            libraryInitialTab = "local";
            navigate("library");
          }}
        />
      {:else if activeTab === "search"}
        <SearchView
          localTracks={library.localTracks}
          {likedIds}
          {cachedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          {proxyPort}
          {externalQuery}
          {externalQuerySeq}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onEnqueue={(track) => audioEngine.enqueue(track)}
          onToggleLike={(track) => void handleToggleLike(track)}
          onDownload={(track) => {
            void downloadTrackOffline(track)
              .then((next) => (library = next))
              .catch(() => {});
          }}
          onOpenPlaylistPicker={(track) => openPlaylistModal(track)}
        />
      {:else if activeTab === "recognize"}
        <RecognizeView
          history={library.recognitionHistory}
          {likedIds}
          {proxyPort}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onSearchQuery={(q) => {
            externalQuery = q;
            externalQuerySeq += 1;
            navigate("search", null);
          }}
          onToggleLike={(track) => void handleToggleLike(track)}
          onHistoryChange={(next) => (library = next)}
          onNotify={showNotice}
        />
      {:else if activeTab === "library"}
        <LibraryView
          {library}
          {selectedPlaylistId}
          tab={libraryInitialTab}
          {likedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          {proxyPort}
          onTabChange={(nextTab) => (libraryInitialTab = nextTab)}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onPlayShuffled={playShuffled}
          onToggleLike={(track) => void handleToggleLike(track)}
          onAddFolder={(path) => {
            void addLocalFolder(path)
              .then((next) => (library = next))
              .catch((e) => {
                playerStore.update((s) => ({ ...s, error: String(e) }));
              });
          }}
          onRemoveFolder={(path) => {
            void removeLocalFolder(path)
              .then((next) => (library = next))
              .catch(() => {});
          }}
          onRescan={() => {
            void rescanLocalFolders()
              .then((next) => (library = next))
              .catch(() => {});
          }}
          onDeleteCached={(id) => {
            void deleteCachedTrack(id)
              .then((next) => (library = next))
              .catch(() => {});
          }}
          onClearCache={() => {
            void clearOfflineCache()
              .then((next) => (library = next))
              .catch(() => {});
          }}
          onSelectPlaylist={(id) => navigate("library", id)}
          onOpenNewPlaylist={() => openPlaylistModal(null)}
          onOpenPlaylistPicker={(track) => openPlaylistModal(track)}
          onRenamePlaylist={async (id, name) => {
            library = await renamePlaylist(id, name);
            showNotice(`Renamed playlist to "${name}"`);
          }}
          onDeletePlaylist={(id) => {
            void deletePlaylist(id).then((next) => {
              library = next;
              selectedPlaylistId = null;
              libraryInitialTab = "playlists";
              showNotice("Playlist deleted");
            });
          }}
          onRemoveFromPlaylist={(plId, trackId) => {
            void removeFromPlaylist(plId, trackId).then(
              (next) => (library = next),
            );
          }}
        />
      {:else if activeTab === "ice"}
        <IceHubView
          {iceStatus}
          {iceSnapshot}
          {iceMeter}
          modePref={library.iceModePref}
          onStatusChange={(st) => (iceStatus = st)}
          onSnapshotChange={(sn) => (iceSnapshot = sn)}
          onModePrefChange={(pref) => (library = { ...library, iceModePref: pref })}
        />
      {:else if activeTab === "settings"}
        <SettingsView
          settings={library.settings}
          {library}
          {iceStatus}
          {iceSnapshot}
          {appPaths}
          onUpdateSettings={(patch) => void handleUpdateSettings(patch)}
          onResetSettings={() => void handleResetSettings()}
          onSetModePref={(pref) => void handleSetModePref(pref)}
          onStatusChange={(st) => (iceStatus = st)}
          onOpenEq={() => (showEq = true)}
          onOpenIceHub={() => navigate("ice", null)}
          onRescanLocal={() => {
            void rescanLocalFolders().then((next) => {
              library = next;
              showNotice(`Scanned ${next.localTracks.length} local tracks`);
            });
          }}
          onClearCache={() => {
            void clearOfflineCache().then((next) => {
              library = next;
              showNotice("Cleared Pandora offline audio cache");
            });
          }}
          onNotify={showNotice}
        />
      {/if}
    </main>
  </div>

  <PlayerBar
    {proxyPort}
    playbackDevices={iceSnapshot?.playback ?? []}
    {likedIds}
    wavyProgress={library.settings.wavyProgress}
    onOpenNowPlaying={() => (showNowPlaying = true)}
    onOpenEq={() => (showEq = true)}
    onToggleLike={(track) => void handleToggleLike(track)}
    onAddToPlaylist={(track) => openPlaylistModal(track)}
  />
</div>

<NowPlayingOverlay
  open={showNowPlaying}
  {proxyPort}
  {likedIds}
  showRomaji={library.settings.showRomaji}
  animatedCanvas={library.settings.animatedCanvas}
  wavyProgress={library.settings.wavyProgress}
  spinningCover={library.settings.spinningCover}
  onClose={() => (showNowPlaying = false)}
  onToggleLike={(track) => void handleToggleLike(track)}
  onAddToPlaylist={(track) => openPlaylistModal(track)}
  onToggleRomaji={() => void handleUpdateSettings({ showRomaji: !library.settings.showRomaji })}
  onToggleCanvas={() =>
    void handleUpdateSettings({ animatedCanvas: !library.settings.animatedCanvas })}
/>

<EqDrawer open={showEq} onClose={() => (showEq = false)} />

<PlaylistModal
  open={playlistModalOpen}
  track={playlistModalTrack}
  playlists={library.playlists}
  {proxyPort}
  onClose={() => (playlistModalOpen = false)}
  onCreatePlaylist={handleCreatePlaylist}
  onToggleTrackInPlaylist={handleToggleTrackInPlaylist}
/>

<style>
  .shell {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .main-area {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    position: relative;
  }

  .toast-err {
    margin: 16px 28px 0;
    padding: 10px 14px;
    border-radius: 12px;
    background: rgba(239, 68, 68, 0.16);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: var(--p-danger);
    font-size: 13px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .toast-ok {
    position: fixed;
    right: 24px;
    bottom: 98px;
    z-index: 70;
    padding: 9px 15px;
    border-radius: 999px;
    background: var(--p-surface-solid);
    border: 1px solid var(--p-accent);
    color: var(--p-accent-bright);
    font-size: 12px;
    font-weight: 600;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
  }
</style>
