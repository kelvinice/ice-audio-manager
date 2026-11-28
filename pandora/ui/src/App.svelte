<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import CollectionView from "./components/CollectionView.svelte";
  import EqDrawer from "./components/EqDrawer.svelte";
  import HomeView from "./components/HomeView.svelte";
  import IceHubView from "./components/IceHubView.svelte";
  import Icon from "./components/Icon.svelte";
  import ImportPlaylistModal from "./components/ImportPlaylistModal.svelte";
  import LibraryView from "./components/LibraryView.svelte";
  import NowPlayingOverlay from "./components/NowPlayingOverlay.svelte";
  import PlayerBar from "./components/PlayerBar.svelte";
  import PlaylistModal from "./components/PlaylistModal.svelte";
  import QueuePanel from "./components/QueuePanel.svelte";
  import RecognizeView from "./components/RecognizeView.svelte";
  import SearchView from "./components/SearchView.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import TitleBar from "./components/TitleBar.svelte";
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
    onLibraryChanged,
    onCompanionRecognized,
    onIceSnapshot,
    onIceStatus,
    removeFromPlaylist,
    removeLocalFolder,
    renamePlaylist,
    rescanLocalFolders,
    resetPandoraSettings,
    saveCollection,
    savePandoraSettings,
    setIceModePref,
    toggleLikeTrack,
    wakeAndPeekIce,
    ytmCollection,
    ytmHome,
  } from "./lib/ipc";
  import { formatCount, shuffled } from "./lib/format";
  import {
    DEFAULT_PANDORA_SETTINGS,
    type AppPaths,
    type CollectionCard,
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
  /** A feed playlist / album opened from Home (shown in place of the Home view). */
  let openCollection = $state<CollectionCard | null>(null);
  let downloading = $state(false);
  let content: HTMLElement | undefined = $state();
  let showNowPlaying = $state(false);
  let showEq = $state(false);
  /** The right-hand queue panel; remembered on this machine (a view preference, not library data). */
  const QUEUE_OPEN_KEY = "pandora.queuePanelOpen";
  let queueOpen = $state(readFlag(QUEUE_OPEN_KEY));

  function readFlag(key: string): boolean {
    try {
      return localStorage.getItem(key) === "1";
    } catch {
      return false;
    }
  }

  function toggleQueue() {
    queueOpen = !queueOpen;
    try {
      localStorage.setItem(QUEUE_OPEN_KEY, queueOpen ? "1" : "0");
    } catch {
      // Storage can be unavailable; the panel still toggles for this session.
    }
  }
  let playlistModalOpen = $state(false);
  let playlistModalTrack = $state<Track | null>(null);
  let importModalOpen = $state(false);
  let statusNotice = $state<string | null>(null);
  let noticeTimer: number | null = null;

  let proxyPort = $state(0);
  let demoMode = $state(false);
  let feedLoading = $state(false);
  /** The title bar's search text (the app's one search box). */
  let searchQuery = $state("");
  /** Bumped by Enter so Search skips its typing pause. */
  let searchSubmitSeq = $state(0);
  let feed = $state<HomeFeed>({
    shelves: [],
    quickPicks: [],
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
  let unlistenLibrary: (() => void) | null = null;
  let unlistenCompanion: (() => void) | null = null;

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
    // Open on the last song (paused where it stopped) instead of an empty player.
    audioEngine.restoreSession(lib.history[0] ?? null);

    if (isTauriEnv()) {
      unlistenStatus = await onIceStatus((st) => (iceStatus = st));
      unlistenSnap = await onIceSnapshot((sn) => (iceSnapshot = sn));
      unlistenMeter = await onIceMeter((fr) => (iceMeter = fr));
      unlistenLibrary = await onLibraryChanged((next) => (library = next));
      unlistenCompanion = await onCompanionRecognized((t) => showNotice(`Ice asked: ${t.title} — ${t.artist}`));
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
    unlistenLibrary?.();
    unlistenCompanion?.();
  });

  async function refreshHome() {
    feedLoading = true;
    try {
      feed = await ytmHome();
    } finally {
      feedLoading = false;
    }
  }

  type LibraryTab = typeof libraryInitialTab;
  interface Route {
    tab: NavTab;
    playlistId: string | null;
    collection: CollectionCard | null;
    libraryTab: LibraryTab;
  }

  const HISTORY_LIMIT = 50;
  let backStack = $state<Route[]>([]);
  let forwardStack = $state<Route[]>([]);

  function currentRoute(): Route {
    return { tab: activeTab, playlistId: selectedPlaylistId, collection: openCollection, libraryTab: libraryInitialTab };
  }

  function applyRoute(route: Route) {
    activeTab = route.tab;
    selectedPlaylistId = route.playlistId;
    openCollection = route.collection;
    libraryInitialTab = route.libraryTab;
  }

  function sameRoute(a: Route, b: Route): boolean {
    return (
      a.tab === b.tab &&
      a.playlistId === b.playlistId &&
      a.collection?.browseId === b.collection?.browseId &&
      (a.tab !== "library" || a.libraryTab === b.libraryTab)
    );
  }

  /** Moves to a new page and records it for Back / Forward. */
  function go(patch: Partial<Route>) {
    const from = currentRoute();
    const to = { ...from, ...patch };
    if (sameRoute(from, to)) {
      applyRoute(to);
      return;
    }
    backStack = [...backStack.slice(1 - HISTORY_LIMIT), from];
    forwardStack = [];
    applyRoute(to);
  }

  function goBack() {
    const to = backStack.at(-1);
    if (!to) return;
    forwardStack = [...forwardStack, currentRoute()];
    backStack = backStack.slice(0, -1);
    applyRoute(to);
  }

  function goForward() {
    const to = forwardStack.at(-1);
    if (!to) return;
    backStack = [...backStack, currentRoute()];
    forwardStack = forwardStack.slice(0, -1);
    applyRoute(to);
  }

  function navigate(tab: NavTab, playlistId: string | null = null) {
    go({ tab, playlistId, collection: null });
  }

  function search(query: string) {
    searchQuery = query;
    if (query.trim() && activeTab !== "search") navigate("search");
  }

  function submitSearch() {
    searchSubmitSeq += 1;
    if (activeTab !== "search") navigate("search");
  }

  // Alt+Left / Alt+Right and the mouse's back / forward buttons, as in a browser.
  function onHistoryKey(e: KeyboardEvent) {
    if (!e.altKey || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
    e.preventDefault();
    if (e.key === "ArrowLeft") goBack();
    else goForward();
  }

  function onHistoryMouse(e: MouseEvent) {
    if (e.button === 3) goBack();
    else if (e.button === 4) goForward();
  }

  // Each page starts at the top, like a new screen.
  $effect(() => {
    void activeTab;
    void selectedPlaylistId;
    void openCollection;
    content?.scrollTo({ top: 0 });
  });

  /** Title bar snowflake: start Ice if needed, wake its notch and show it for a moment. */
  async function peekIce() {
    try {
      iceStatus = await wakeAndPeekIce();
    } catch (e) {
      showNotice(e instanceof Error ? e.message : String(e));
    }
  }

  async function playCollection(card: CollectionCard) {
    try {
      const page = await ytmCollection(card.browseId);
      if (page.tracks.length > 0) void audioEngine.playTrack(page.tracks[0], page.tracks);
    } catch (e) {
      showNotice(`Couldn't open "${card.title}": ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleSaveCollection(name: string, tracks: Track[]) {
    try {
      const [next] = await saveCollection(name, tracks);
      library = next;
      showNotice(`Saved "${name}" to your library`);
    } catch (e) {
      showNotice(`Couldn't save: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function downloadOne(track: Track) {
    try {
      library = await downloadTrackOffline(track);
      showNotice(`Downloaded "${track.title}"`);
    } catch (e) {
      showNotice(`Couldn't download "${track.title}": ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  /** Downloads one song at a time so playback and the feed keep their bandwidth. */
  async function downloadAll(tracks: Track[]) {
    if (downloading || tracks.length === 0) return;
    downloading = true;
    let failed = 0;
    try {
      for (const [i, track] of tracks.entries()) {
        showNotice(`Downloading ${i + 1} of ${tracks.length}: ${track.title}`);
        try {
          library = await downloadTrackOffline(track);
        } catch {
          failed += 1;
        }
      }
    } finally {
      downloading = false;
    }
    const done = tracks.length - failed;
    showNotice(failed > 0 ? `Downloaded ${formatCount(done, "song")}, ${failed} failed` : `Downloaded ${formatCount(done, "song")}`);
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
    const order = shuffled(list);
    void audioEngine.playTrack(order[0], order);
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

<svelte:window onkeydown={onHistoryKey} onmouseup={onHistoryMouse} />

<div class="shell">
  <TitleBar
    query={searchQuery}
    onHome={activeTab === "home" && !openCollection}
    onSearchPage={activeTab === "search"}
    canBack={backStack.length > 0}
    canForward={forwardStack.length > 0}
    iceLive={iceStatus.bridgeConnected || iceStatus.mode === "embedded"}
    onBack={goBack}
    onForward={goForward}
    onGoHome={() => navigate("home")}
    onSearch={search}
    onSubmitSearch={submitSearch}
    onIdentify={() => navigate("recognize")}
    onOpenIce={() => void peekIce()}
    onOpenSettings={() => navigate("settings")}
    onOpenEq={() => (showEq = true)}
    onNewPlaylist={() => openPlaylistModal(null)}
    onImportPlaylist={() => (importModalOpen = true)}
  />
  <div class="main-area">
    <Sidebar
      {activeTab}
      libraryTab={libraryInitialTab}
      {selectedPlaylistId}
      {library}
      {proxyPort}
      {iceStatus}
      {defaultDevice}
      showIceCard={library.settings.showIceCardInSidebar}
      onNavigate={navigate}
      onOpenLibraryTab={(libraryTab) => go({ tab: "library", playlistId: null, collection: null, libraryTab })}
      onOpenNewPlaylist={() => openPlaylistModal(null)}
      onOpenImportPlaylist={() => (importModalOpen = true)}
    />

    <main class="content" class:beside-panel={queueOpen} bind:this={content}>
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

      {#if activeTab === "home" && openCollection}
        <CollectionView
          card={openCollection}
          {likedIds}
          {cachedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          isPlaying={$playerStore.playing}
          {proxyPort}
          onBack={() => go({ collection: null })}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onPlayShuffled={playShuffled}
          onToggleLike={(track) => void handleToggleLike(track)}
          onAddToPlaylist={(track) => openPlaylistModal(track)}
          onDownload={(track) => void downloadOne(track)}
          onDownloadAll={(tracks) => void downloadAll(tracks)}
          onSave={(name, tracks) => void handleSaveCollection(name, tracks)}
        />
      {:else if activeTab === "home"}
        <HomeView
          {feed}
          loading={feedLoading}
          {demoMode}
          history={library.history}
          {likedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          isPlaying={$playerStore.playing}
          {proxyPort}
          onPlay={(track, list) => void audioEngine.playTrack(track, list)}
          onToggleLike={(track) => void handleToggleLike(track)}
          onOpenPlaylistPicker={(track) => openPlaylistModal(track)}
          onOpenCollection={(card) => go({ collection: card })}
          onPlayCollection={(card) => void playCollection(card)}
          onRefresh={() => void refreshHome()}
          onOpenLocal={() => {
            go({ tab: "library", playlistId: null, collection: null, libraryTab: "local" });
          }}
        />
      {:else if activeTab === "search"}
        <SearchView
          localTracks={library.localTracks}
          {likedIds}
          {cachedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          {proxyPort}
          query={searchQuery}
          submitSeq={searchSubmitSeq}
          isPlaying={$playerStore.playing}
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
            searchQuery = q;
            submitSearch();
          }}
          onToggleLike={(track) => void handleToggleLike(track)}
          onAddToPlaylist={(track) => openPlaylistModal(track)}
          onHistoryChange={(next) => (library = next)}
          onNotify={showNotice}
        />
      {:else if activeTab === "library"}
        <LibraryView
          {library}
          {selectedPlaylistId}
          tab={libraryInitialTab}
          {likedIds}
          {cachedIds}
          currentTrackId={$playerStore.currentTrack?.id ?? null}
          isPlaying={$playerStore.playing}
          {proxyPort}
          onTabChange={(nextTab) => go({ libraryTab: nextTab, playlistId: null })}
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
          onDownload={(track) => void downloadOne(track)}
          onDownloadAll={(tracks) => void downloadAll(tracks)}
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
          onSelectPlaylist={(id) => {
            if (id === null) go({ tab: "library", playlistId: null, collection: null, libraryTab: "playlists" });
            else navigate("library", id);
          }}
          onOpenNewPlaylist={() => openPlaylistModal(null)}
          onOpenImportPlaylist={() => (importModalOpen = true)}
          onOpenPlaylistPicker={(track) => openPlaylistModal(track)}
          onRenamePlaylist={async (id, name) => {
            library = await renamePlaylist(id, name);
            showNotice(`Renamed playlist to "${name}"`);
          }}
          onDeletePlaylist={(id) => {
            void deletePlaylist(id).then((next) => {
              library = next;
              go({ playlistId: null, libraryTab: "playlists" });
              showNotice("Playlist deleted");
            });
          }}
          onRemoveFromPlaylist={(plId, trackId) => {
            void removeFromPlaylist(plId, trackId).then(
              (next) => (library = next),
            );
          }}
          onSynced={(next, message) => {
            library = next;
            showNotice(message);
          }}
          onAddToThisPlaylist={(plId, track) => void handleToggleTrackInPlaylist(plId, track, false)}
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
              showNotice("Removed all downloads");
            });
          }}
          onOpenDownloads={() => go({ tab: "library", playlistId: null, collection: null, libraryTab: "offline" })}
          onNotify={showNotice}
        />
      {/if}
    </main>

    {#if queueOpen}
      <QueuePanel history={library.history} {proxyPort} onClose={toggleQueue} />
    {/if}
  </div>

  <PlayerBar
    {proxyPort}
    playbackDevices={iceSnapshot?.playback ?? []}
    {likedIds}
    wavyProgress={library.settings.wavyProgress}
    onOpenNowPlaying={() => (showNowPlaying = true)}
    onOpenEq={() => (showEq = true)}
    {queueOpen}
    onToggleQueue={toggleQueue}
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
  backdropMotion={library.settings.backdropMotion}
  pulse={library.settings.experimental.beatPulse}
  wavyProgress={library.settings.wavyProgress}
  spinningCover={library.settings.spinningCover}
  onClose={() => (showNowPlaying = false)}
  onToggleLike={(track) => void handleToggleLike(track)}
  onAddToPlaylist={(track) => openPlaylistModal(track)}
  onToggleRomaji={() => void handleUpdateSettings({ showRomaji: !library.settings.showRomaji })}
  lyricsStyle={library.settings.lyricsStyle}
  onSetLyricsStyle={(lyricsStyle) => void handleUpdateSettings({ lyricsStyle })}
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

<ImportPlaylistModal
  open={importModalOpen}
  onClose={() => (importModalOpen = false)}
  onImported={(res) => {
    library = res.library;
    showNotice(`Imported "${res.playlistName}" (${res.importedTracks} tracks)`);
  }}
  onOpenPlaylist={(id) => {
    importModalOpen = false;
    navigate("library", id);
  }}
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

  /* The title bar, the gaps and the player bar form one frame; sidebar panels and the page sit on it. */
  .shell {
    background: var(--p-rail);
  }

  .main-area {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 8px;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    position: relative;
    margin: 0 8px 8px 0;
    border-radius: 10px;
    background: radial-gradient(ellipse at 72% -30%, var(--p-glow-1), transparent 55%), var(--p-bg);
  }

  /* The queue panel brings its own right margin; the 8 px gap separates them. */
  .content.beside-panel {
    margin-right: 0;
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
