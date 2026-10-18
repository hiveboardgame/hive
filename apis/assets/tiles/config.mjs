// Run with SVGO 4: npx svgo@4.0.0 --config apis/assets/tiles/config.mjs --input <file.svg>
const paintServers = new Set(['linearGradient', 'radialGradient', 'filter']);

// Only remove unreachable paint resources. Public piece/overlay fragment IDs remain intact.
const removeUnusedPaintServers = {
  name: 'removeUnusedPaintServers',
  fn(root) {
    const resources = new Map();
    function collect(node) {
      if (node.type === 'element' && paintServers.has(node.name) && node.attributes.id) {
        resources.set(node.attributes.id, node);
      }
      for (const child of node.children ?? []) collect(child);
    }
    collect(root);
    const live = new Set();
    const pending = [];
    function references(node, skipResources) {
      if (skipResources && node.type === 'element' && paintServers.has(node.name)) return;
      for (const [name, value] of Object.entries(node.attributes ?? {})) {
        if ((name === 'href' || name === 'xlink:href') && value.startsWith('#')) pending.push(value.slice(1));
        for (const match of value.matchAll(/url\(['"]?#([^)'"\s]+)['"]?\)/g)) pending.push(match[1]);
      }
      for (const child of node.children ?? []) references(child, skipResources);
    }
    references(root, true);
    while (pending.length) {
      const id = pending.pop();
      if (live.has(id)) continue;
      live.add(id);
      if (resources.has(id)) references(resources.get(id), false);
    }
    function prune(node) {
      if (!node.children) return;
      node.children = node.children.filter(child => !(child.type === 'element' && paintServers.has(child.name) && child.attributes.id && !live.has(child.attributes.id)));
      for (const child of node.children) prune(child);
    }
    prune(root);
    return {};
  },
};

export default {
  // Keep geometry, numeric precision, transforms, group compositing and fragment IDs.
  // The default preset may round paths or remove IDs used by external <use> callers.
  plugins: [
    'removeDoctype',
    'removeXMLProcInst',
    'removeComments',
    'removeMetadata',
    'removeEditorsNSData',
    'cleanupAttrs',
    'convertStyleToAttrs',
    removeUnusedPaintServers,
    'removeUnusedNS',
  ],
  js2svg: { pretty: false },
};
