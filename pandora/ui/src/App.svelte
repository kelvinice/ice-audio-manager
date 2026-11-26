<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import EqDrawer from "./components/EqDrawer.svelte";
  import HomeView from "./components/HomeView.svelte";
  import IceHubView from "./components/IceHubView.svelte";
  import LibraryView from "./components/LibraryView.svelte";
  import NowPlayingOverlay from "./components/NowPlayingOverlay.svelte";
  import PlayerBar from "./components/PlayerBar.svelte";
  import SearchView from "./components/SearchView.svelte";
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
    getIceSnapshot,
    getIceStatus,
    getLibrary,
    getProxyPort,
    isTauriEnv,
    onIceMeter,
    onIceSnapshot,
    onIceStatus,
    removeFromPlaylist,
    removeLocalFolder,
    rescanLocalFolders,
    toggleLikeTrack,
    ytmHome,
  } from "./lib/ipc";
  import type {
    EngineSnapshot,
    HomeFeed,
    IceStatus,
    LibraryState,
    MeterFrame,
    Track,
  } from "./lib/types";

  let activeTab = $state<NavTab>("home");
  let selectedPlaylistId = $state<string | null>(null);
  let showNowPlaying = $state(false);
  let showEq = $state(false);

  let proxyPort = $state(0);
  let feedLoading = $state(false);
  let feed = $state<HomeFeed>({ quickPicks: [], trending: [], offlineFallback: false, error: null });
  let library = $state<LibraryState>({
    liked: [],
    playlists: [],
    history: [],
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
  });

  let iceStatus = $state<IceStatus>({
    mode: "unavailable",
    standaloneRunning: false,
    standaloneExe: null,
    configPath: "",
    error: null,
  });
  let iceSnapshot = $state<EngineSnapshot | null>(null);
  let iceMeter = $state<MeterFrame | null>(null);

  let unlistenStatus: (() => void) | null = null;
  let unlistenSnap: (() => void) | null = null;
  let unlistenMeter: (() => void) | null = null;

  let likedIds = $derived(new Set(library.liked.map((t) => t.id)));
  let cachedIds = $derived(new Set(library.cachedIds));
  let defaultDevice = $derived(iceSnapshot?.playback.find((d) => d.isDefault));

  onMount(async () => {
    proxyPort = await getProxyPort().catch(() => 0);
    const lib = await getLibrary().catch(() => library);
    library = lib;
    audioEngine.init(proxyPort, lib.eq, () => {
      void getLibrary().then((next) => (library = next)).catch(() => {});
    });

    if (isTauriEnv()) {
      unlistenStatus = await onIceStatus((st) => (iceStatus = st));
      unlistenSnap = await onIceSnapshot((sn) => (iceSnapshot = sn));
      unlistenMeter = await onIceMeter((fr) => (iceMeter = fr));
      void getIceStatus().then((st) => (iceStatus = st)).catch(() => {});
      void getIceSnapshot().then((sn) => (iceSnapshot = sn)).catch(() => {});
    }

    await refreshHome();
  });

  onDestroy(() => {
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

  async function handleToggleLike(track: Track) {
    library = await toggleLikeTrack(track);
    if ($playerStore.currentTrack?.id === track.id) {
      playerStore.update((s) =>
        s.currentTrack ? { ...s, currentTrack: { ...s.currentTrack, liked: !s.currentTrack.liked } } : s,
      );
    }
  }
</script>

<div class="ambient-bg">
  <div class="ambient-blob b1"></div>
  <div class="ambient-blob b2"></div>
  <div class="ambient-blob b3"></div>
</div>

<div class="shell">
  <div class="main-area">
    <Sidebar
      {activeTab}
      {selectedPlaylistId}
      playlists={library.playlists}
      {iceStatus}
      {defaultDevice}
      onNavigate={navigate}
      onCreatePlaylist={(name) => {
        void createPlaylist(name).then((next) => (library = next)).catch(() => {});
      }}
    />

    <main class="content">
      {#if $playerStore.error}
        <div class="toast-err">
          <span>{$playerStore.error}</span>
          <button onclick={() => playerStore.update((s) => ({ ...s, error: null }))}>✕</button>
        </div>
      {/if}

      {#if activeTab === "home"}
        <HomeView
          {feed}
          loading={feedLoading}
          history={library.history}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          {proxyPort}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onRefresh={() => void refreshHome()}
          onGoSearch={() => navigate("search")}
        />
      {:else if activeTab === "search"}
        <SearchView
          localTracks={library.localTracks}
          {likedIds}
          {cachedIds}
          playlists={library.playlists}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          {proxyPort}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onEnqueue={(track) => audioEngine.enqueue(track)}
          onToggleLike={(track) => void handleToggleLike(track)}
          onDownload={(track) => {
            void downloadTrackOffline(track).then((next) => (library = next)).catch(() => {});
          }}
          onAddToPlaylist={(plId, track) => {
            void addToPlaylist(plId, track).then((next) => (library = next)).catch(() => {});
          }}
        />
      {:else if activeTab === "library"}
        <LibraryView
          {library}
          {selectedPlaylistId}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          {proxyPort}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onToggleLike={(track) => void handleToggleLike(track)}
          onAddFolder={(path) => {
            void addLocalFolder(path).then((next) => (library = next)).catch(() => {});
          }}
          onRemoveFolder={(path) => {
            void removeLocalFolder(path).then((next) => (library = next)).catch(() => {});
          }}
          onRescan={() => {
            void rescanLocalFolders().then((next) => (library = next)).catch(() => {});
          }}
          onDeleteCached={(id) => {
            void deleteCachedTrack(id).then((next) => (library = next)).catch(() => {});
          }}
          onClearCache={() => {
            void clearOfflineCache().then((next) => (library = next)).catch(() => {});
          }}
          onDeletePlaylist={(id) => {
            void deletePlaylist(id).then((next) => {
              library = next;
              selectedPlaylistId = null;
            });
          }}
          onRemoveFromPlaylist={(plId, trackId) => {
            void removeFromPlaylist(plId, trackId).then((next) => (library = next));
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
        />
      {/if}
    </main>
  </div>

  <PlayerBar
    {proxyPort}
    playbackDevices={iceSnapshot?.playback ?? []}
    onOpenNowPlaying={() => (showNowPlaying = true)}
    onOpenEq={() => (showEq = true)}
    onToggleLike={(track) => void handleToggleLike(track)}
  />
</div>

<NowPlayingOverlay
  open={showNowPlaying}
  {proxyPort}
  onClose={() => (showNowPlaying = false)}
  onToggleLike={(track) => void handleToggleLike(track)}
/>

<EqDrawer open={showEq} onClose={() => (showEq = false)} />

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
    color: #fecaca;
    font-size: 13px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
</style>
