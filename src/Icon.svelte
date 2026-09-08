<script lang="ts">
  export let name = 'file';
  export let size = 20;
  const paths: Record<string, string> = {
    crown: 'm3 6 4 4 5-7 5 7 4-4-2 13H5L3 6ZM7 22h10',
    grid: 'M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z',
    layers: 'm12 3 10 5-10 5L2 8l10-5ZM2 12l10 5 10-5M2 16l10 5 10-5',
    route: 'M6 5h11a4 4 0 0 1 0 8H7a4 4 0 0 0 0 8h11M3 2h6v6H3zM18 18l3 3-3 3',
    file: 'M14 2H5v20h14V7l-5-5Zm0 0v6h5M8 12h8M8 16h6',
    folder: 'M3 6h7l2 3h9v11H3V6ZM3 6V4h7l2 2h7v3',
    arrow: 'M4 12h16m-6-6 6 6-6 6',
    check: 'm5 12 4 4L19 6',
    shield: 'm12 2 9 4v6c0 5-9 10-9 10S3 17 3 12V6l9-4Zm-5 10 3 3 7-7',
    download: 'M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4',
    search: 'M21 21l-6-6M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0Z',
    plus: 'M12 5v14M5 12h14',
    clock: 'M12 8v5l3 2M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z',
    info: 'M12 11v6M12 7h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z',
    lock: 'M5 10h14v11H5zM8 10V6a4 4 0 0 1 8 0v4',
    chevron: 'm9 5 7 7-7 7',
    x: 'm6 6 12 12M6 18 18 6'
  };
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={paths[name] || paths.file} /></svg>
