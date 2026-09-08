<script lang="ts">
  import Icon from './Icon.svelte';
  import type { InspectedSave, Slot, ConversionMode, ConversionSummary } from './types';

  let tab: 'overview' | 'structure' | 'plan' = 'overview';
  let source: InspectedSave | null = null;
  let reference: InspectedSave | null = null;
  let busy: Slot | 'export' | 'convert' | null = null;
  let error = '';
  let notice = '';
  let query = '';
  let conversionMode: ConversionMode = 'experimental-random-regions-1.16.1-to-1.19.0.6';
  let conversion: ConversionSummary | null = null;
  $: canConvert = !!source && !busy && desktop && (conversionMode === 'roundtrip' || version === '1.16.1') && (conversionMode !== 'experimental-random-regions-1.16.1-to-1.19.0.6' || meta(reference, 'version') === '1.19.0.6');
  async function convertSave() {
    if (!window.continuum || !canConvert) return;
    busy = 'convert'; error = ''; notice = ''; conversion = null;
    try {
      const reply = await window.continuum.convertSave(conversionMode);
      if (!reply.ok) {
        if (reply.cancelled) notice = 'Conversion cancelled. No completed output was published.';
        else error = reply.error;
      } else if (reply.value) {
        conversion = reply.value;
        notice = 'Test save created: ' + reply.value.outputName + '. Engine compatibility is still unverified.';
      }
    } catch { error = 'Conversion connection interrupted. Check the destination for any incomplete staging folder.'; }
    finally { busy = null; }
  }
  const desktop = Boolean(window.continuum);
  const number = (n: number) => n.toLocaleString();
  const bytes = (n: number) => `${(n / 1024 / 1024).toFixed(1)} MB`;
  const meta = (save: InspectedSave | null, key: string) => save?.inspection.metadata.find(f => f.key === key)?.value || '—';
  $: version = meta(source, 'version');
  $: sourceSections = new Map(source?.inspection.sections.map(s => [s.key, s]) || []);
  $: referenceSections = new Map(reference?.inspection.sections.map(s => [s.key, s]) || []);
  $: sectionNames = [...new Set([...sourceSections.keys(), ...referenceSections.keys()])].sort();
  $: filteredSections = sectionNames.filter(key => key.toLowerCase().includes(query.toLowerCase()));
  $: rootCount = source?.inspection.sections.reduce((sum, section) => sum + section.occurrences, 0) || 0;
  $: integrity = source?.inspection.crcVerified ? 'ZIP integrity verified' : 'Text scan completed';

  async function openSave(slot: Slot) {
    if (!window.continuum || busy) return;
    busy = slot; error = ''; notice = '';
    try {
      const reply = await window.continuum.openSave(slot);
      if (!reply.ok) {
        if (reply.cancelled) notice = 'Inspection cancelled. Your previous results are still available.';
        else error = reply.error;
      } else if (reply.value) {
        if (slot === 'source') { source = reply.value; conversion = null; }
        else reference = reply.value;
        notice = `${reply.value.name} inspected. Original save unchanged.`;
      }
    } catch { error = 'The desktop connection was interrupted. Reopen the app and try again.'; }
    finally { busy = null; }
  }
  async function cancel() {
    const reply = await window.continuum?.cancelInspection();
    if (reply && !reply.ok) error = reply.error;
  }
  async function exportReport() {
    if (!window.continuum || busy) return;
    busy = 'export'; error = ''; notice = '';
    try {
      const reply = await window.continuum.exportReport();
      if (!reply.ok) error = reply.error;
      else if (reply.value) notice = `Report saved as ${reply.value}. It includes save names and campaign metadata.`;
    } catch { error = 'Could not export the report. Please try again.'; }
    finally { busy = null; }
  }
  const milestones = [
    { name: 'Inspect the save', state: 'Available', body: 'Read the container, verify compressed data, and inventory the campaign without changing it.' },
    { name: 'Preserve every untouched byte', state: 'Testable', body: 'Create a separate save with byte-preservation checks, synchronized metadata, and a change journal. In-game testing remains required.' },
    { name: 'Reconcile the world', state: 'Research', body: 'Resolve map changes, title and trait mappings, generated cultures, and mod dependencies.' },
    { name: 'Prove continuation in game', state: 'Planned', body: 'Load, advance, save, and reload in the target engine. Test succession, travel, and long-term simulation.' }
  ];
</script>

<svelte:head><title>CK3 Continuum — {tab === 'overview' ? 'Campaign workspace' : tab === 'structure' ? 'Save structure' : 'Migration plan'}</title></svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand"><div class="brand-mark"><Icon name="crown" size={25} /></div><div><strong>CONTINUUM</strong><span>CRUSADER KINGS III</span></div></div>
    <div class="workspace-label">YOUR WORKSPACE</div>
    <nav aria-label="Workspace">
      <button class:active={tab === 'overview'} onclick={() => tab = 'overview'}><Icon name="grid" />Overview</button>
      <button class:active={tab === 'structure'} onclick={() => tab = 'structure'}><Icon name="layers" />Save structure{#if source}<span class="nav-count">{source.inspection.sections.length}</span>{/if}</button>
      <button class:active={tab === 'plan'} onclick={() => tab = 'plan'}><Icon name="route" />Migration plan</button>
    </nav>
    <div class="sidebar-note"><span class="tiny-label">BUILT FOR CONTINUATION</span><p>Every campaign has<br />more stories to tell.</p><div class="rule-decoration"><span></span>✦<span></span></div></div>
    <div class="sidebar-footer"><span class="status-dot"></span>Local processing only<div>Prototype <span>v0.3.1</span></div></div>
  </aside>

  <div class="workspace">
    <header class="topbar"><div><span>Workspace</span><Icon name="chevron" size={13} /><strong>{tab === 'overview' ? 'Overview' : tab === 'structure' ? 'Save structure' : 'Migration plan'}</strong></div><span class="prototype-badge">RESEARCH PROTOTYPE</span></header>
    <main>
      <div class="page-heading"><div><div class="eyebrow">THE NEXT CHAPTER</div><h1>{tab === 'overview' ? 'Your legacy, continued.' : tab === 'structure' ? 'Inside your campaign.' : 'The road to migration.'}</h1><p>{tab === 'overview' ? 'Understand your old save. Prepare for a new era.' : tab === 'structure' ? 'Explore what is present, repeated, and different across your saves.' : 'A careful path from an old campaign to a playable modern save.'}</p></div><button class="button secondary export-button" disabled={!source || !!busy} onclick={exportReport}><Icon name="download" size={17} />Export report</button></div>

      {#if !desktop}<div class="message warning"><Icon name="info" /><div><strong>Browser preview</strong><p>Open the Electron app to inspect local saves. This preview shows the interface only.</p></div></div>{/if}
      {#if error}<div class="message error" role="alert"><Icon name="info" /><div><strong>Operation could not be completed</strong><p>{error}</p><small>Your original saves have not been modified.</small></div><button class="icon-button" aria-label="Dismiss error" onclick={() => error = ''}><Icon name="x" size={16} /></button></div>{/if}
      <div class="sr-only" role="status" aria-live="polite">{busy ? 'Working. Reading and verifying save data.' : notice}</div>
      {#if notice}<div class="notice"><Icon name="check" size={15} />{notice}</div>{/if}
      {#if busy && busy !== 'export'}<div class="progress-card"><span class="spinner"></span><div><strong>{busy === 'convert' ? 'Creating and verifying your test save…' : 'Reading and verifying your ' + (busy === 'source' ? 'campaign' : 'reference') + '…'}</strong><span>Large saves can take longer. Your original remains unchanged.</span></div><button class="button subtle" onclick={cancel}>Cancel</button></div>{/if}

      {#if tab === 'overview'}
        <section class="campaign-panel" aria-label="Campaign save">
          <div class="campaign-art" aria-hidden="true"><div class="orbit orbit-one"></div><div class="orbit orbit-two"></div><div class="art-crown"><Icon name="crown" size={58} /></div><span class="art-cross c1">+</span><span class="art-cross c2">+</span><span class="art-cross c3">+</span></div>
          <div class="campaign-content"><span class="tiny-label gold">01 / YOUR CAMPAIGN</span><h2>{source ? (meta(source, 'meta_player_name') !== '—' ? meta(source, 'meta_player_name') : 'Campaign inspected') : 'Bring your history with you.'}</h2><p>{source ? source.name : 'Start with a .ck3 save. We’ll read its version and structure, and help you understand what a migration will need.'}</p>
            <div class="campaign-actions"><button class="button primary" data-testid="open-source" disabled={!desktop || !!busy} onclick={() => openSave('source')}><Icon name="folder" size={18} />{source ? 'Choose another save' : 'Open campaign save'}<Icon name="arrow" size={17} /></button><span>.ck3 · text or compressed text</span></div>
            <div class="preserve-note"><Icon name="shield" size={15} />Your original save always stays untouched.</div>
          </div>
        </section>

        <div class="summary-grid">
          <article class="stat"><div class="stat-label">SAVE VERSION<Icon name="file" size={16} /></div><strong data-testid="source-version">{source ? version : '—'}</strong><span>{source ? 'Read from embedded metadata' : 'Waiting for a campaign'}</span></article>
          <article class="stat"><div class="stat-label">CAMPAIGN DATE<Icon name="clock" size={16} /></div><strong>{meta(source, 'meta_date')}</strong><span>{source ? meta(source, 'meta_title_name') : 'Your story so far'}</span></article>
          <article class="stat"><div class="stat-label">SAVE SIZE<Icon name="layers" size={16} /></div><strong>{source ? bytes(source.inspection.fileBytes) : '—'}</strong><span>{source ? `${bytes(source.inspection.gamestateBytes)} uncompressed` : 'Measured during inspection'}</span></article>
        </div>

        <div class="details-grid">
          <section class="card reference-card"><div class="card-title"><div class="small-icon"><Icon name="layers" /></div><div><h3>Reference save</h3><span>Required for new regions</span></div><span class="pill">02</span></div><p>A save from your target version gives you a structural reference. Campaign history and mods can also explain differences.</p>
            {#if reference}<div class="reference-loaded"><Icon name="check" size={17} /><div><strong>{reference.name}</strong><span>Version {meta(reference, 'version')} · {bytes(reference.inspection.fileBytes)}</span></div></div>{/if}
            <button class="button secondary full" data-testid="open-reference" disabled={!desktop || !!busy} onclick={() => openSave('reference')}><Icon name={reference ? 'folder' : 'plus'} size={16} />{reference ? 'Replace reference save' : 'Add reference save'}</button>
          </section>
          <section class="card readiness-card"><div class="card-title"><div class="small-icon amber"><Icon name="route" /></div><div><h3>Migration readiness</h3><span>Research target · CK3 1.19.0.6</span></div></div><div class="readiness-status"><span class="amber-dot"></span>{source ? 'Compatibility needs investigation' : 'Begin with an inspection'}</div><p>{source ? 'A successful scan confirms readable data. Map changes, mod content, and in-game behavior still need validation.' : 'Inspect a campaign, then open the migration plan to create an experimental test save or a writer-control copy.'}</p><button class="text-button" onclick={() => tab = 'plan'}>View the migration plan<Icon name="arrow" size={16} /></button></section>
        </div>

        <section class="card inspection-card"><div class="section-heading"><div><h3>Inspection summary</h3><p>{source ? 'Measured from your save, without rewriting it.' : 'Your campaign’s details will appear here.'}</p></div>{#if source}<span class="verified"><Icon name="check" size={14} />{integrity}</span>{:else}<span class="pill">AWAITING SAVE</span>{/if}</div>
          {#if source}<div class="inspection-grid"><div><span>Distinct root keys</span><strong>{number(source.inspection.sections.length)}</strong></div><div><span>Root occurrences</span><strong>{number(rootCount)}</strong></div><div><span>Tokens scanned</span><strong>{number(source.inspection.tokenCount)}</strong></div><div><span>Native scan time</span><strong>{number(source.inspection.elapsedMs)} ms</strong></div></div><button class="text-button" onclick={() => tab = 'structure'}>Explore save structure<Icon name="arrow" size={16} /></button>{:else}<div class="empty-inspection"><Icon name="file" size={30} /><div><strong>No campaign inspected yet</strong><span>Open a save above to see its metadata and structure.</span></div><span class="empty-line"></span></div>{/if}
        </section>
      {:else if tab === 'structure'}
        {#if !source}<section class="card empty-state"><Icon name="layers" size={36} /><h2>A closer look starts with a save.</h2><p>Inspect a campaign to browse its root sections and metadata.</p><button class="button primary" disabled={!desktop || !!busy} onclick={() => openSave('source')}>Open campaign save<Icon name="arrow" size={16} /></button></section>{:else}
          <div class="message warning"><Icon name="info" /><div><strong>Structure is evidence, not a migration recipe.</strong><p>Counts describe immediate entries, including lists and tombstones. A missing section or a different count does not prove a required version change.</p></div></div>
          <section class="card structure-card"><div class="section-heading"><div><h3>Root sections <span class="inline-count">{sectionNames.length}</span></h3><p>{reference ? 'Campaign and reference inventories, side by side.' : 'Add a reference from Overview to compare inventories.'}</p></div><label class="search"><Icon name="search" size={17} /><input aria-label="Filter sections" placeholder="Filter sections…" bind:value={query} /></label></div>
            <div class="table-scroll"><table><thead><tr><th>Section</th><th>Campaign entries</th><th>Occurrences</th>{#if reference}<th>Reference entries</th><th>Difference</th>{/if}</tr></thead><tbody>{#each filteredSections as key}<tr><td><code>{key}</code></td><td>{sourceSections.has(key) ? number(sourceSections.get(key)!.childEntries) : 'Absent'}</td><td>{sourceSections.has(key) ? number(sourceSections.get(key)!.occurrences) : '—'}</td>{#if reference}<td>{referenceSections.has(key) ? number(referenceSections.get(key)!.childEntries) : 'Absent'}</td><td><span class="delta">{sourceSections.has(key) && referenceSections.has(key) ? number(referenceSections.get(key)!.childEntries - sourceSections.get(key)!.childEntries) : 'Different presence'}</span></td>{/if}</tr>{/each}</tbody></table>{#if !filteredSections.length}<p class="no-results">No sections match “{query}”.</p>{/if}</div>
          </section>
          <section class="card metadata-card"><h3>Campaign metadata</h3><p>Ordered scalar fields from the inner metadata block.</p><dl>{#each source.inspection.metadata as field}<div><dt>{field.key}</dt><dd>{field.value}</dd></div>{/each}</dl><div class="hash"><span>Decompressed gamestate SHA-256</span><code>{source.inspection.gamestateSha256}</code></div></section>
        {/if}
      {:else}
        <div class="plan-intro"><Icon name="shield" size={28} /><div><h2>Preserve the campaign. Prove the result.</h2><p>The goal is a playable continuation, including the world, family, culture, and history. A version-label edit alone cannot establish that.</p></div></div>
        <section class="card milestones">{#each milestones as item, index}<div class="milestone"><div class:completed={index === 0} class="milestone-number">{#if index === 0}<Icon name="check" size={18} />{:else}0{index + 1}{/if}</div><div><div class="milestone-title"><h3>{item.name}</h3><span class:available={index === 0} class="pill">{item.state}</span></div><p>{item.body}</p></div></div>{/each}</section>
        <div class="details-grid"><section class="card"><h3>Known research areas</h3><ul class="research-list"><li>Province and title identity across changed maps</li><li>Trait indices, faith and religion schema</li><li>Generated cultures and administrative systems</li><li>Source mods and missing content definitions</li></ul><p class="muted">These are findings from the 1.16.1 → 1.19.0.6 research pair, not automatically detected issues in your save.</p></section><section class="card conversion-card">
          <Icon name="route" size={25} /><h3>Create a test save</h3><span class="pill">EXPERIMENTAL · ENGINE UNVERIFIED</span>
          <p>Writes a separate .ck3 file and a conversion report. Your source stays untouched.</p>
          <label class="conversion-label" for="conversion-mode">Conversion profile</label>
          <select id="conversion-mode" bind:value={conversionMode} disabled={!!busy}>
            <option value="experimental-random-regions-1.16.1-to-1.19.0.6">1.16.1 -> 1.19.0.6: random regional kingdoms</option>
            <option value="experimental-1.16.1-to-1.19.0.6">1.16.1 → 1.19.0.6 · structural test</option>
            <option value="roundtrip">Writer control · no migration</option>
          </select>
          {#if conversionMode === 'roundtrip'}
            <p>Rebuilds the supported container with identical gamestate bytes. Use this to isolate writer behavior from migration behavior.</p>
          {:else if conversionMode === 'experimental-random-regions-1.16.1-to-1.19.0.6'}
            <p>Registers missing regions and creates new families in independent feudal kingdoms with county vassals. Existing rulers, families, and cultures are preserved. Requires a 1.19.0.6 reference save for geography and regional defaults.</p>
            <p>Advanced eastern governments and changes to existing provinces still need migration. Test portraits and new realms in CK3 before continuing the campaign.</p>
            {#if meta(reference, 'version') !== '1.19.0.6'}<p class="conversion-warning">Open a 1.19.0.6 reference save from Overview.</p>{/if}
          {:else}
            <p>Renames religion/faith template fields, groups title display names, and updates the version label. Map, trait, portrait, mod, and system migration remain unresolved.</p>
            {#if source && version !== '1.16.1'}<p class="conversion-warning">This profile requires a 1.16.1 source save.</p>{/if}
          {/if}
          <button class="button primary full" data-testid="convert-save" disabled={!canConvert} onclick={convertSave}><Icon name="arrow" size={16} />{conversionMode === 'roundtrip' ? 'Create writer-control save' : 'Convert save'}</button>
          {#if !source}<p>Open a campaign from Overview to begin.</p>{/if}
        </section></div>
      {/if}
      {#if conversion}<section class="card conversion-result" aria-label="Conversion result">
        <span class="verified"><Icon name="check" size={14} />Output structure verified · engine untested</span>
        <h3>{conversion.outputName}</h3><p>Report: {conversion.reportName} · {bytes(conversion.outputBytes)}</p>
        <ul>{#each Object.entries(conversion.counts) as [rule, count]}<li>{rule}: {number(count)} changed spans</li>{/each}</ul>
        <p>{conversion.profile === 'roundtrip' ? 'Gamestate bytes are unchanged.' : 'Unchanged byte ranges were verified; engine behavior still needs testing.'}</p>
        <details><summary>Unresolved work and in-game test steps</summary><ul>{#each conversion.warnings as warning}<li>{warning}</li>{/each}</ul>
        <p>First test the modern writer-control copy in CK3. Then load this candidate, inspect the player and world, advance one day and one month, save, exit, and reload. Keep separate logs and copies for each test.</p></details>
      </section>{/if}
      <footer class="workspace-footer"><span><Icon name="shield" size={13} />Originals preserved. Test outputs are separate.</span><span>Unofficial community project</span></footer>
    </main>
  </div>
</div>
