/* @ds-bundle: {"format":3,"namespace":"CelimosDesignSystem_555e0b","components":[{"name":"ACCENT_PRESETS","sourcePath":"components/core/AccentPicker.jsx"},{"name":"AccentPicker","sourcePath":"components/core/AccentPicker.jsx"},{"name":"Avatar","sourcePath":"components/core/Avatar.jsx"},{"name":"Badge","sourcePath":"components/core/Badge.jsx"},{"name":"ExplicitBadge","sourcePath":"components/core/Badge.jsx"},{"name":"QualityBadge","sourcePath":"components/core/Badge.jsx"},{"name":"Button","sourcePath":"components/core/Button.jsx"},{"name":"Chip","sourcePath":"components/core/Chip.jsx"},{"name":"Icon","sourcePath":"components/core/Icon.jsx"},{"name":"IconButton","sourcePath":"components/core/IconButton.jsx"},{"name":"Input","sourcePath":"components/core/Input.jsx"},{"name":"NumberStepper","sourcePath":"components/core/NumberStepper.jsx"},{"name":"PlayButton","sourcePath":"components/core/PlayButton.jsx"},{"name":"SegmentedControl","sourcePath":"components/core/SegmentedControl.jsx"},{"name":"Select","sourcePath":"components/core/Select.jsx"},{"name":"SettingsGroup","sourcePath":"components/core/SettingsGroup.jsx"},{"name":"SettingsRow","sourcePath":"components/core/SettingsGroup.jsx"},{"name":"Slider","sourcePath":"components/core/Slider.jsx"},{"name":"Switch","sourcePath":"components/core/Switch.jsx"},{"name":"Tabs","sourcePath":"components/core/Tabs.jsx"},{"name":"AlbumTile","sourcePath":"components/media/AlbumTile.jsx"},{"name":"CoverArt","sourcePath":"components/media/CoverArt.jsx"},{"name":"TrackRow","sourcePath":"components/media/TrackRow.jsx"}],"sourceHashes":{"components/core/AccentPicker.jsx":"90afebba71bd","components/core/Avatar.jsx":"b97f21ad65fb","components/core/Badge.jsx":"2d7f32ff32d4","components/core/Button.jsx":"871f2d30ebc3","components/core/Chip.jsx":"590d316164ab","components/core/Icon.jsx":"9a4d7a13f118","components/core/IconButton.jsx":"9f323105c475","components/core/Input.jsx":"0ae4652de4ca","components/core/NumberStepper.jsx":"52ead190948f","components/core/PlayButton.jsx":"31feedc09d53","components/core/SegmentedControl.jsx":"6969dda9bb33","components/core/Select.jsx":"cf18ee0ab23a","components/core/SettingsGroup.jsx":"f38756d4732c","components/core/Slider.jsx":"24b64d204ea3","components/core/Switch.jsx":"21310bec410d","components/core/Tabs.jsx":"2ab9e17136ff","components/media/AlbumTile.jsx":"336a6ef8a33e","components/media/CoverArt.jsx":"e8e8d36242f3","components/media/TrackRow.jsx":"d4450f2c2494","ui_kits/celimos-app/AppShell.jsx":"c3156de468c7","ui_kits/celimos-app/ArtistScreen.jsx":"9148e3a386f6","ui_kits/celimos-app/CollectionScreen.jsx":"ae546054b420","ui_kits/celimos-app/Common.jsx":"dc0853855e20","ui_kits/celimos-app/HomeScreen.jsx":"2ec5f13d1b4c","ui_kits/celimos-app/LibraryScreen.jsx":"bcb820f6ff21","ui_kits/celimos-app/NowPlaying.jsx":"5ccc4b61774a","ui_kits/celimos-app/QueuePanel.jsx":"4ad7862c9442","ui_kits/celimos-app/SearchScreen.jsx":"31bff2d77b06","ui_kits/celimos-app/SettingsScreen.jsx":"32b926bb0e22","ui_kits/celimos-app/data.js":"f26fb9086bbb","ui_kits/celimos-app/preferences.js":"0b57945d074b"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.CelimosDesignSystem_555e0b = window.CelimosDesignSystem_555e0b || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/core/Avatar.jsx
try { (() => {
/* Avatar — artist circle. Shows an image when given, else gradient initials.
   `square` with a small radius is used for user/profile in the sidebar. */

function hashHue(str = '') {
  let h = 0;
  for (let i = 0; i < str.length; i++) h = (h * 31 + str.charCodeAt(i)) % 360;
  return h;
}
function Avatar({
  name = '',
  src,
  size = 40,
  shape = 'round',
  ring = false,
  style = {}
}) {
  const initials = name.split(' ').filter(Boolean).slice(0, 2).map(w => w[0]).join('').toUpperCase() || '♪';
  const hue = hashHue(name);
  const radius = shape === 'square' ? 'var(--radius-md)' : '50%';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      width: size,
      height: size,
      borderRadius: radius,
      flex: 'none',
      overflow: 'hidden',
      position: 'relative',
      background: src ? 'var(--ink-700)' : `linear-gradient(140deg, hsl(${hue} 55% 32%), hsl(${(hue + 40) % 360} 50% 18%))`,
      display: 'grid',
      placeItems: 'center',
      boxShadow: ring ? '0 0 0 2px var(--bg-window), 0 0 0 3px var(--accent)' : 'none',
      ...style
    }
  }, src ? /*#__PURE__*/React.createElement("img", {
    src: src,
    alt: name,
    style: {
      width: '100%',
      height: '100%',
      objectFit: 'cover'
    }
  }) : /*#__PURE__*/React.createElement("span", {
    style: {
      font: `var(--weight-bold) ${size * 0.36}px/1 var(--font-display)`,
      color: 'var(--on-art)',
      letterSpacing: '-0.02em'
    }
  }, initials));
}
Object.assign(__ds_scope, { Avatar });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Avatar.jsx", error: String((e && e.message) || e) }); }

// components/core/Badge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* Badge — tiny inline status/quality markers. Variants tuned to a music client:
   explicit (E), lossless / hi-res quality, "NEW", plus generic tones. */

const TONES = {
  neutral: {
    bg: 'var(--ink-700)',
    fg: 'var(--text-secondary)',
    bd: 'var(--line)'
  },
  accent: {
    bg: 'var(--teal-tint)',
    fg: 'var(--teal-400)',
    bd: 'transparent'
  },
  success: {
    bg: 'color-mix(in srgb, var(--success) 16%, transparent)',
    fg: 'var(--success)',
    bd: 'transparent'
  },
  warning: {
    bg: 'color-mix(in srgb, var(--warning) 16%, transparent)',
    fg: 'var(--warning)',
    bd: 'transparent'
  },
  danger: {
    bg: 'color-mix(in srgb, var(--danger) 16%, transparent)',
    fg: 'var(--danger)',
    bd: 'transparent'
  },
  outline: {
    bg: 'transparent',
    fg: 'var(--text-secondary)',
    bd: 'var(--line-strong)'
  }
};
function Badge({
  children,
  tone = 'neutral',
  size = 'md',
  uppercase = false,
  style = {},
  ...rest
}) {
  const t = TONES[tone] || TONES.neutral;
  const pad = size === 'sm' ? '1px 6px' : '2px 9px';
  const fs = size === 'sm' ? 10 : 11;
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      padding: pad,
      borderRadius: 'var(--radius-sm)',
      background: t.bg,
      color: t.fg,
      border: `1px solid ${t.bd}`,
      font: `var(--weight-semibold) ${fs}px/1 var(--font-ui)`,
      letterSpacing: uppercase ? 'var(--tracking-wide)' : 'var(--tracking-tight)',
      textTransform: uppercase ? 'uppercase' : 'none',
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), children);
}

/* The explicit [E] mark — square, muted, used inline next to track titles. */
function ExplicitBadge({
  size = 16,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("span", {
    title: "Explicit",
    style: {
      display: 'inline-grid',
      placeItems: 'center',
      width: size,
      height: size,
      borderRadius: 'var(--radius-xs)',
      background: 'var(--ink-600)',
      color: 'var(--text-secondary)',
      font: `var(--weight-bold) ${size * 0.62}px/1 var(--font-ui)`,
      flex: 'none',
      ...style
    }
  }, "E");
}

/* Lossless / Hi-Res quality pill seen on now-playing & track rows. */
function QualityBadge({
  level = 'Lossless',
  style = {}
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      padding: '2px 8px',
      borderRadius: 'var(--radius-sm)',
      border: '1px solid var(--teal-700)',
      color: 'var(--teal-400)',
      font: 'var(--weight-bold) 10px/1 var(--font-ui)',
      letterSpacing: 'var(--tracking-wide)',
      textTransform: 'uppercase',
      ...style
    }
  }, level);
}
Object.assign(__ds_scope, { Badge, ExplicitBadge, QualityBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Badge.jsx", error: String((e && e.message) || e) }); }

// components/core/Chip.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* Chip — selectable pill used for filters / genre tags / "moods". Toggles
   between resting and selected (accent-tinted). */

function Chip({
  children,
  selected = false,
  onClick,
  size = 'md',
  disabled = false,
  style = {},
  ...rest
}) {
  const [hover, setHover] = React.useState(false);
  const h = size === 'sm' ? 28 : 34;
  const pad = size === 'sm' ? '0 12px' : '0 16px';
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    onClick: onClick,
    disabled: disabled,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    "aria-pressed": selected,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      height: h,
      padding: pad,
      borderRadius: 'var(--radius-pill)',
      font: 'var(--text-label-font)',
      whiteSpace: 'nowrap',
      cursor: disabled ? 'not-allowed' : 'pointer',
      background: selected ? 'var(--accent)' : hover ? 'var(--ink-700)' : 'var(--ink-800)',
      color: selected ? 'var(--text-on-accent)' : 'var(--text-secondary)',
      border: `1px solid ${selected ? 'transparent' : 'var(--line)'}`,
      fontWeight: selected ? 700 : 500,
      opacity: disabled ? 0.4 : 1,
      transition: 'var(--t-hover)',
      ...style
    }
  }, rest), children);
}
Object.assign(__ds_scope, { Chip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Chip.jsx", error: String((e && e.message) || e) }); }

// components/core/Icon.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* Celimos icon set.
   Outline icons follow Lucide geometry (ISC licensed, 24×24, stroke 2, round caps).
   Transport glyphs (play/pause/skip) are filled — universal player symbols.
   Usage: <Icon name="play" size={20} /> · stroke icons inherit currentColor. */

const STROKE = {
  search: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "11",
    cy: "11",
    r: "8"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m21 21-4.3-4.3"
  })),
  home: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 9.5 12 3l9 6.5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M5 10v10h14V10"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M9 20v-6h6v6"
  })),
  compass: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "9"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m15.5 8.5-2 5-5 2 2-5 5-2Z"
  })),
  library: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 5h14"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3 10h14"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3 15h9"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M19 6v13"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m16 16 3 3 3-3"
  })),
  disc: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "9"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "2.5"
  })),
  mic: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "9",
    y: "3",
    width: "6",
    height: "11",
    rx: "3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M5 11a7 7 0 0 0 14 0"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 18v3"
  })),
  heart: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M19.5 5.6a4.6 4.6 0 0 0-6.5 0L12 6.6l-1-1a4.6 4.6 0 1 0-6.5 6.5l1 1L12 20l6.5-6.9 1-1a4.6 4.6 0 0 0 0-6.5Z"
  })),
  plus: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M12 5v14"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M5 12h14"
  })),
  check: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "m20 6-11 11-5-5"
  })),
  checkCircle: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "9"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m8.5 12 2.5 2.5 4.5-5"
  })),
  download: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M12 3v12"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m7 11 5 5 5-5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M4 20h16"
  })),
  clock: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "9"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 7v5l3.5 2"
  })),
  list: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M8 6h13"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M8 12h13"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M8 18h13"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3.5 6h.01"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3.5 12h.01"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3.5 18h.01"
  })),
  queue: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 6h12"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3 12h12"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3 18h8"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m16 9 5 3-5 3V9Z"
  })),
  more: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "5",
    cy: "12",
    r: "1.4"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "1.4"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "19",
    cy: "12",
    r: "1.4"
  })),
  moreV: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "5",
    r: "1.4"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "1.4"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "19",
    r: "1.4"
  })),
  volume: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M11 5 6 9H3v6h3l5 4V5Z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M16 9a4 4 0 0 1 0 6"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M19 6a8 8 0 0 1 0 12"
  })),
  volumeLow: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M11 5 6 9H3v6h3l5 4V5Z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M16 9.5a3.5 3.5 0 0 1 0 5"
  })),
  volumeMute: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M11 5 6 9H3v6h3l5 4V5Z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m22 9-5 6"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m17 9 5 6"
  })),
  chevronLeft: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "m15 6-6 6 6 6"
  })),
  chevronRight: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "m9 6 6 6-6 6"
  })),
  chevronDown: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "m6 9 6 6 6-6"
  })),
  x: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M6 6 18 18"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M18 6 6 18"
  })),
  settings: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2"
  })),
  airplay: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M5 17H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-1"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m12 15 5 6H7l5-6Z"
  })),
  share: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "18",
    cy: "5",
    r: "3"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "6",
    cy: "12",
    r: "3"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "18",
    cy: "19",
    r: "3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m8.6 13.5 6.8 4M15.4 6.5 8.6 10.5"
  })),
  sort: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M11 5h10"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M11 9h7"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M11 13h4"
  }), /*#__PURE__*/React.createElement("path", {
    d: "m3 8 3-3 3 3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M6 5v14"
  })),
  filter: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 5h18l-7 8v6l-4-2v-4L3 5Z"
  })),
  star: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "m12 3 2.6 5.6 6 .7-4.4 4.1 1.2 6L12 16.6 6.6 19.4l1.2-6L3.4 9.3l6-.7L12 3Z"
  })),
  sparkles: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "m12 4 1.6 4.4L18 10l-4.4 1.6L12 16l-1.6-4.4L6 10l4.4-1.6L12 4Z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M19 15l.7 2 2 .7-2 .7-.7 2-.7-2-2-.7 2-.7.7-2Z"
  })),
  grid: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "3",
    width: "7",
    height: "7",
    rx: "1.5"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "14",
    y: "3",
    width: "7",
    height: "7",
    rx: "1.5"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "14",
    width: "7",
    height: "7",
    rx: "1.5"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "14",
    y: "14",
    width: "7",
    height: "7",
    rx: "1.5"
  })),
  rows: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "4",
    width: "18",
    height: "5",
    rx: "1.5"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "13",
    width: "18",
    height: "5",
    rx: "1.5"
  })),
  server: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "4",
    width: "18",
    height: "7",
    rx: "2"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "13",
    width: "18",
    height: "7",
    rx: "2"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M7 7.5h.01M7 16.5h.01"
  })),
  lyrics: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M4 6h16M4 11h16M4 16h9"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "18",
    cy: "17",
    r: "2.5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M20.5 17V11l-3 1"
  }))
};
const FILLED = {
  play: /*#__PURE__*/React.createElement("path", {
    d: "M7 4.5v15a1 1 0 0 0 1.5.86l13-7.5a1 1 0 0 0 0-1.72l-13-7.5A1 1 0 0 0 7 4.5Z"
  }),
  pause: /*#__PURE__*/React.createElement("path", {
    d: "M7 4h3.5v16H7zM13.5 4H17v16h-3.5z"
  }),
  skipBack: /*#__PURE__*/React.createElement("path", {
    d: "M7 5a1 1 0 0 1 2 0v5.2l9.4-5.9A1 1 0 0 1 20 5.2v13.6a1 1 0 0 1-1.6.8L9 13.7V19a1 1 0 0 1-2 0V5Z"
  }),
  skipForward: /*#__PURE__*/React.createElement("path", {
    d: "M17 5a1 1 0 0 0-2 0v5.2L5.6 4.3A1 1 0 0 0 4 5.2v13.6a1 1 0 0 0 1.6.8L15 13.7V19a1 1 0 0 0 2 0V5Z"
  }),
  shuffle: /*#__PURE__*/React.createElement("path", {
    d: "M16 4h4v4M20 4l-6 6M16 20h4v-4M20 20l-5-5M4 5l4.5 4.5M4 19l11-11",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }),
  repeat: /*#__PURE__*/React.createElement("path", {
    d: "M17 2.5 21 6l-4 3.5M21 6H8a4 4 0 0 0-4 4v1M7 21.5 3 18l4-3.5M3 18h13a4 4 0 0 0 4-4v-1",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }),
  repeatOne: /*#__PURE__*/React.createElement("g", null, /*#__PURE__*/React.createElement("path", {
    d: "M17 2.5 21 6l-4 3.5M21 6H8a4 4 0 0 0-4 4v1M7 21.5 3 18l4-3.5M3 18h13a4 4 0 0 0 4-4v-1",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }), /*#__PURE__*/React.createElement("text", {
    x: "12",
    y: "15.5",
    fontSize: "8",
    fontWeight: "700",
    textAnchor: "middle",
    fill: "currentColor",
    stroke: "none"
  }, "1")),
  heartFill: /*#__PURE__*/React.createElement("path", {
    d: "M19.5 5.6a4.6 4.6 0 0 0-7.5 1l-1-1a4.6 4.6 0 1 0-6.5 6.5l1 1L12 20l6.5-6.9 1-1a4.6 4.6 0 0 0 0-6.5Z"
  }),
  explicit: /*#__PURE__*/React.createElement("g", null, /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "3",
    width: "18",
    height: "18",
    rx: "4"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M9 8h6M9 12h5M9 16h6M9 8v8",
    fill: "none",
    stroke: "var(--ink-950, #0a0a0c)",
    strokeWidth: "1.8",
    strokeLinecap: "round"
  }))
};
function Icon({
  name,
  size = 20,
  strokeWidth = 1.9,
  className = '',
  style = {},
  ...rest
}) {
  const filled = FILLED[name];
  const stroke = STROKE[name];
  const content = filled || stroke;
  if (!content) return null;
  return /*#__PURE__*/React.createElement("svg", _extends({
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: filled ? 'currentColor' : 'none',
    stroke: filled ? 'none' : 'currentColor',
    strokeWidth: filled ? undefined : strokeWidth,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    className: className,
    style: {
      flex: 'none',
      display: 'block',
      ...style
    },
    "aria-hidden": "true"
  }, rest), content);
}
Object.assign(__ds_scope, { Icon });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Icon.jsx", error: String((e && e.message) || e) }); }

// components/core/AccentPicker.jsx
try { (() => {
/* AccentPicker — swatch row for choosing the brand accent (the Feishin-style
   "accent color" control). Built-in themes map to data-accent values; an
   optional custom swatch opens a native color input. The selected value is the
   theme key ('teal' | 'indigo' | … | 'custom'); customColor carries the hex. */

const ACCENT_PRESETS = [{
  key: 'teal',
  color: '#00d9c0',
  label: 'Teal'
}, {
  key: 'indigo',
  color: '#6d72ff',
  label: 'Indigo'
}, {
  key: 'emerald',
  color: '#2ee08a',
  label: 'Emerald'
}, {
  key: 'amber',
  color: '#ff9f43',
  label: 'Amber'
}, {
  key: 'gold',
  color: '#d6b069',
  label: 'Gold'
}, {
  key: 'crimson',
  color: '#ff5470',
  label: 'Crimson'
}];
function AccentPicker({
  value = 'teal',
  customColor = '#8b5cff',
  onChange,
  allowCustom = true,
  style = {}
}) {
  const Swatch = ({
    k,
    color,
    label
  }) => {
    const on = value === k;
    return /*#__PURE__*/React.createElement("button", {
      type: "button",
      title: label,
      "aria-label": label,
      "aria-pressed": on,
      onClick: () => onChange && onChange(k, k === 'custom' ? customColor : color),
      style: {
        width: 30,
        height: 30,
        borderRadius: '50%',
        cursor: 'pointer',
        position: 'relative',
        background: color,
        border: 'none',
        boxShadow: on ? `0 0 0 2px var(--bg-window), 0 0 0 4px ${color}` : '0 1px 3px rgba(0,0,0,0.35)',
        transform: on ? 'scale(1.04)' : 'scale(1)',
        transition: 'var(--t-hover)'
      }
    }, on && /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        inset: 0,
        display: 'grid',
        placeItems: 'center',
        color: '#0a0a0c'
      }
    }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: "check",
      size: 15,
      strokeWidth: 2.6
    })));
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      flexWrap: 'wrap',
      ...style
    }
  }, ACCENT_PRESETS.map(p => /*#__PURE__*/React.createElement(Swatch, {
    key: p.key,
    k: p.key,
    color: p.color,
    label: p.label
  })), allowCustom && /*#__PURE__*/React.createElement("label", {
    title: "Custom color",
    style: {
      width: 30,
      height: 30,
      borderRadius: '50%',
      cursor: 'pointer',
      position: 'relative',
      overflow: 'hidden',
      display: 'grid',
      placeItems: 'center',
      background: value === 'custom' ? customColor : 'conic-gradient(from 0deg, #ff5470, #ff9f43, #2ee08a, #00d9c0, #6d72ff, #ff5470)',
      boxShadow: value === 'custom' ? `0 0 0 2px var(--bg-window), 0 0 0 4px ${customColor}` : '0 1px 3px rgba(0,0,0,0.35)',
      transform: value === 'custom' ? 'scale(1.04)' : 'scale(1)',
      transition: 'var(--t-hover)'
    }
  }, /*#__PURE__*/React.createElement("input", {
    type: "color",
    value: customColor,
    onChange: e => onChange && onChange('custom', e.target.value),
    style: {
      position: 'absolute',
      inset: 0,
      opacity: 0,
      cursor: 'pointer',
      width: '100%',
      height: '100%'
    }
  }), value === 'custom' ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "check",
    size: 15,
    strokeWidth: 2.6,
    style: {
      color: '#0a0a0c'
    }
  }) : /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "plus",
    size: 15,
    strokeWidth: 2.4,
    style: {
      color: '#fff',
      mixBlendMode: 'difference'
    }
  })));
}
Object.assign(__ds_scope, { ACCENT_PRESETS, AccentPicker });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/AccentPicker.jsx", error: String((e && e.message) || e) }); }

// components/core/Button.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* Celimos Button — primary action uses the teal accent; most chrome buttons
   are secondary (subtle fill) or ghost (transparent). Pill-shaped. */

const SIZES = {
  sm: {
    height: 'var(--control-h-sm)',
    padding: '0 14px',
    font: 'var(--text-label-font)',
    icon: 15,
    gap: 6
  },
  md: {
    height: 'var(--control-h-md)',
    padding: '0 18px',
    font: 'var(--text-label-font)',
    icon: 17,
    gap: 8
  },
  lg: {
    height: 'var(--control-h-lg)',
    padding: '0 26px',
    font: 'var(--text-title-font)',
    icon: 19,
    gap: 9
  }
};
const VARIANTS = {
  primary: {
    background: 'var(--accent)',
    color: 'var(--text-on-accent)',
    border: '1px solid transparent',
    fontWeight: 700,
    '--hov': 'var(--accent-hover)',
    '--act': 'var(--accent-press)',
    boxShadow: '0 2px 14px var(--focus-ring)'
  },
  secondary: {
    background: 'var(--ink-700)',
    color: 'var(--text-primary)',
    border: '1px solid var(--line)',
    '--hov': 'var(--ink-600)',
    '--act': 'var(--ink-750)'
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text-secondary)',
    border: '1px solid transparent',
    '--hov': 'var(--ink-750)',
    '--act': 'var(--ink-800)'
  },
  outline: {
    background: 'transparent',
    color: 'var(--text-primary)',
    border: '1px solid var(--line-strong)',
    '--hov': 'var(--ink-800)',
    '--act': 'var(--ink-750)'
  },
  danger: {
    background: 'transparent',
    color: 'var(--danger)',
    border: '1px solid color-mix(in srgb, var(--danger) 40%, transparent)',
    '--hov': 'color-mix(in srgb, var(--danger) 14%, transparent)',
    '--act': 'color-mix(in srgb, var(--danger) 22%, transparent)'
  }
};
function Button({
  children,
  variant = 'secondary',
  size = 'md',
  iconLeft,
  iconRight,
  disabled = false,
  loading = false,
  fullWidth = false,
  style = {},
  ...rest
}) {
  const s = SIZES[size] || SIZES.md;
  const v = VARIANTS[variant] || VARIANTS.secondary;
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);
  const bg = active ? v['--act'] : hover ? v['--hov'] : v.background;
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    disabled: disabled || loading,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setActive(false);
    },
    onMouseDown: () => setActive(true),
    onMouseUp: () => setActive(false),
    style: {
      display: fullWidth ? 'flex' : 'inline-flex',
      width: fullWidth ? '100%' : 'auto',
      alignItems: 'center',
      justifyContent: 'center',
      gap: s.gap,
      height: s.height,
      padding: s.padding,
      font: s.font,
      fontWeight: v.fontWeight || 'var(--weight-medium)',
      letterSpacing: 'var(--tracking-tight)',
      background: bg,
      color: v.color,
      border: v.border,
      borderRadius: 'var(--radius-pill)',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.4 : 1,
      whiteSpace: 'nowrap',
      userSelect: 'none',
      boxShadow: variant === 'primary' && !disabled ? v.boxShadow : 'none',
      transform: active && !disabled ? 'scale(0.97)' : 'scale(1)',
      transition: 'var(--t-hover)',
      ...style
    }
  }, rest), loading ? /*#__PURE__*/React.createElement(Spinner, {
    size: s.icon
  }) : iconLeft && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: iconLeft,
    size: s.icon
  }), children && /*#__PURE__*/React.createElement("span", null, children), iconRight && !loading && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: iconRight,
    size: s.icon
  }));
}
function Spinner({
  size
}) {
  return /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    style: {
      animation: 'celspin 0.7s linear infinite'
    }
  }, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "9",
    stroke: "currentColor",
    strokeOpacity: "0.25",
    strokeWidth: "3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M21 12a9 9 0 0 0-9-9",
    stroke: "currentColor",
    strokeWidth: "3",
    strokeLinecap: "round"
  }), /*#__PURE__*/React.createElement("style", null, `@keyframes celspin{to{transform:rotate(360deg)}}`));
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Button.jsx", error: String((e && e.message) || e) }); }

// components/core/IconButton.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* IconButton — square/round transparent control used all over the chrome:
   transport secondary controls, like, more-menu, nav arrows.
   `active` makes it adopt the accent color (e.g. shuffle/repeat engaged). */

const SIZES = {
  sm: {
    box: 30,
    icon: 17
  },
  md: {
    box: 38,
    icon: 20
  },
  lg: {
    box: 46,
    icon: 24
  }
};
function IconButton({
  name,
  size = 'md',
  variant = 'ghost',
  active = false,
  shape = 'round',
  disabled = false,
  title,
  style = {},
  ...rest
}) {
  const s = SIZES[size] || SIZES.md;
  const [hover, setHover] = React.useState(false);
  const [press, setPress] = React.useState(false);
  let bg = 'transparent';
  if (variant === 'solid') bg = hover ? 'var(--ink-600)' : 'var(--ink-700)';else if (hover) bg = 'var(--ink-750)';
  if (press) bg = 'var(--ink-800)';
  const color = active ? 'var(--accent)' : variant === 'solid' ? 'var(--text-primary)' : hover ? 'var(--text-primary)' : 'var(--text-secondary)';
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    title: title,
    "aria-label": title,
    "aria-pressed": active,
    disabled: disabled,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setPress(false);
    },
    onMouseDown: () => setPress(true),
    onMouseUp: () => setPress(false),
    style: {
      display: 'inline-grid',
      placeItems: 'center',
      width: s.box,
      height: s.box,
      flex: 'none',
      background: bg,
      color,
      border: 'none',
      borderRadius: shape === 'round' ? '50%' : 'var(--radius-md)',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.35 : 1,
      transform: press && !disabled ? 'scale(0.92)' : 'scale(1)',
      transition: 'var(--t-hover)',
      position: 'relative',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: name,
    size: s.icon
  }), active && /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      bottom: 3,
      left: '50%',
      transform: 'translateX(-50%)',
      width: 3,
      height: 3,
      borderRadius: '50%',
      background: 'var(--accent)'
    }
  }));
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/core/Input.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* Input — text field used for search and settings forms. Optional leading icon
   and clear button. Dark sunken fill, teal focus ring. */

function Input({
  value,
  onChange,
  placeholder,
  icon,
  type = 'text',
  size = 'md',
  clearable = false,
  onClear,
  disabled = false,
  style = {},
  inputStyle = {},
  ...rest
}) {
  const [focus, setFocus] = React.useState(false);
  const h = size === 'sm' ? 'var(--control-h-sm)' : size === 'lg' ? 'var(--control-h-lg)' : 'var(--control-h-md)';
  const iconSize = size === 'lg' ? 20 : 18;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      height: h,
      padding: '0 14px',
      width: '100%',
      background: 'var(--ink-1000)',
      border: `1px solid ${focus ? 'var(--accent)' : 'var(--line)'}`,
      borderRadius: 'var(--radius-pill)',
      boxShadow: focus ? '0 0 0 3px var(--focus-ring)' : 'none',
      transition: 'var(--t-hover)',
      opacity: disabled ? 0.5 : 1,
      ...style
    }
  }, icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: iconSize,
    style: {
      color: focus ? 'var(--text-secondary)' : 'var(--text-tertiary)'
    }
  }), /*#__PURE__*/React.createElement("input", _extends({
    type: type,
    value: value,
    placeholder: placeholder,
    disabled: disabled,
    onChange: e => onChange && onChange(e.target.value),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      flex: 1,
      minWidth: 0,
      background: 'transparent',
      border: 'none',
      outline: 'none',
      color: 'var(--text-primary)',
      font: 'var(--text-body-font)',
      ...inputStyle
    }
  }, rest)), clearable && value && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onClear,
    "aria-label": "Clear",
    style: {
      display: 'grid',
      placeItems: 'center',
      width: 20,
      height: 20,
      border: 'none',
      borderRadius: '50%',
      background: 'var(--ink-600)',
      color: 'var(--text-secondary)',
      cursor: 'pointer',
      flex: 'none'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "x",
    size: 13
  })));
}
Object.assign(__ds_scope, { Input });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Input.jsx", error: String((e && e.message) || e) }); }

// components/core/NumberStepper.jsx
try { (() => {
/* NumberStepper — compact −/value/+ control for sizes & counts (grid card size,
   crossfade seconds, columns). Clamps to min/max; optional unit suffix. */

function NumberStepper({
  value = 0,
  min = 0,
  max = 100,
  step = 1,
  unit = '',
  onChange,
  disabled = false,
  width = 132,
  style = {}
}) {
  const clamp = v => Math.max(min, Math.min(max, v));
  const set = v => onChange && onChange(clamp(v));
  const Btn = ({
    d,
    name
  }) => /*#__PURE__*/React.createElement("button", {
    type: "button",
    disabled: disabled || (d < 0 ? value <= min : value >= max),
    onClick: () => set(value + d * step),
    style: {
      display: 'grid',
      placeItems: 'center',
      width: 30,
      height: '100%',
      flex: 'none',
      border: 'none',
      background: 'transparent',
      color: 'var(--text-secondary)',
      cursor: 'pointer',
      opacity: (d < 0 ? value <= min : value >= max) ? 0.3 : 1,
      transition: 'var(--t-hover)'
    },
    onMouseEnter: e => {
      e.currentTarget.style.color = 'var(--text-primary)';
    },
    onMouseLeave: e => {
      e.currentTarget.style.color = 'var(--text-secondary)';
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: name,
    size: 16
  }));
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      height: 'var(--control-h-md)',
      width,
      background: 'var(--ink-1000)',
      border: '1px solid var(--line)',
      borderRadius: 'var(--radius-md)',
      opacity: disabled ? 0.5 : 1,
      ...style
    }
  }, /*#__PURE__*/React.createElement(Btn, {
    d: -1,
    name: "chevronLeft"
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      textAlign: 'center',
      font: 'var(--text-label-font)',
      fontFamily: 'var(--font-mono)',
      color: 'var(--text-primary)'
    }
  }, value, unit), /*#__PURE__*/React.createElement(Btn, {
    d: 1,
    name: "chevronRight"
  }));
}
Object.assign(__ds_scope, { NumberStepper });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/NumberStepper.jsx", error: String((e && e.message) || e) }); }

// components/core/PlayButton.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* PlayButton — the signature circular accent control. Used as the big FAB on
   album/artist headers, the transport center button, and (in `mini`) on hover
   over album tiles. Springy press. */

const SIZES = {
  sm: {
    box: 36,
    icon: 16
  },
  md: {
    box: 48,
    icon: 20
  },
  lg: {
    box: 56,
    icon: 24
  },
  xl: {
    box: 64,
    icon: 28
  }
};
function PlayButton({
  playing = false,
  size = 'lg',
  tone = 'accent',
  onToggle,
  disabled = false,
  style = {},
  ...rest
}) {
  const s = SIZES[size] || SIZES.lg;
  const [hover, setHover] = React.useState(false);
  const [press, setPress] = React.useState(false);
  const accent = tone === 'accent';
  const bg = accent ? hover ? 'var(--accent-hover)' : 'var(--accent)' : hover ? 'var(--on-art)' : 'rgba(255,255,255,0.92)';
  const fg = accent ? 'var(--text-on-accent)' : '#000';
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    "aria-label": playing ? 'Pause' : 'Play',
    disabled: disabled,
    onClick: onToggle,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setPress(false);
    },
    onMouseDown: () => setPress(true),
    onMouseUp: () => setPress(false),
    style: {
      display: 'grid',
      placeItems: 'center',
      flex: 'none',
      width: s.box,
      height: s.box,
      borderRadius: '50%',
      background: bg,
      color: fg,
      border: 'none',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.4 : 1,
      boxShadow: accent ? hover ? '0 8px 30px var(--focus-ring)' : '0 4px 18px var(--focus-ring)' : '0 6px 22px rgba(0,0,0,0.5)',
      transform: press ? 'scale(0.9)' : hover ? 'scale(1.06)' : 'scale(1)',
      transition: 'transform var(--dur-fast) var(--ease-spring), background-color var(--dur-fast) var(--ease-standard), box-shadow var(--dur-fast) var(--ease-standard)',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: playing ? 'pause' : 'play',
    size: s.icon,
    style: {
      marginLeft: playing ? 0 : s.box * 0.04
    }
  }));
}
Object.assign(__ds_scope, { PlayButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/PlayButton.jsx", error: String((e && e.message) || e) }); }

// components/core/SegmentedControl.jsx
try { (() => {
/* SegmentedControl — pill-grouped exclusive choice (Albums / Artists / Tracks).
   Sliding accent-tinted thumb behind the active segment. */

function SegmentedControl({
  options = [],
  value,
  onChange,
  size = 'md',
  style = {}
}) {
  const idx = Math.max(0, options.findIndex(o => (o.value ?? o) === value));
  const h = size === 'sm' ? 30 : 36;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'inline-flex',
      position: 'relative',
      padding: 3,
      height: h,
      background: 'var(--ink-1000)',
      borderRadius: 'var(--radius-pill)',
      border: '1px solid var(--line)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 3,
      bottom: 3,
      left: 3,
      width: `calc((100% - 6px) / ${options.length})`,
      transform: `translateX(${idx * 100}%)`,
      background: 'var(--ink-700)',
      borderRadius: 'var(--radius-pill)',
      boxShadow: 'var(--edge-top)',
      transition: 'transform var(--dur-base) var(--ease-out)'
    }
  }), options.map(o => {
    const val = o.value ?? o;
    const label = o.label ?? o;
    const on = val === value;
    return /*#__PURE__*/React.createElement("button", {
      key: val,
      type: "button",
      onClick: () => onChange && onChange(val),
      style: {
        position: 'relative',
        zIndex: 1,
        flex: 1,
        border: 'none',
        background: 'transparent',
        padding: '0 16px',
        cursor: 'pointer',
        whiteSpace: 'nowrap',
        font: 'var(--text-label-font)',
        fontWeight: on ? 600 : 500,
        color: on ? 'var(--text-primary)' : 'var(--text-tertiary)',
        transition: 'color var(--dur-fast)'
      }
    }, label);
  }));
}
Object.assign(__ds_scope, { SegmentedControl });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/SegmentedControl.jsx", error: String((e && e.message) || e) }); }

// components/core/Select.jsx
try { (() => {
/* Select — custom dropdown (not native) so it matches the dark chrome. Used
   throughout settings: theme, default view, sort order, etc. */

function Select({
  value,
  options = [],
  onChange,
  size = 'md',
  placeholder = 'Select…',
  disabled = false,
  align = 'left',
  style = {}
}) {
  const [open, setOpen] = React.useState(false);
  const ref = React.useRef(null);
  const h = size === 'sm' ? 'var(--control-h-sm)' : size === 'lg' ? 'var(--control-h-lg)' : 'var(--control-h-md)';
  React.useEffect(() => {
    if (!open) return;
    const onDoc = e => {
      if (ref.current && !ref.current.contains(e.target)) setOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, [open]);
  const opts = options.map(o => typeof o === 'string' ? {
    label: o,
    value: o
  } : o);
  const sel = opts.find(o => o.value === value);
  return /*#__PURE__*/React.createElement("div", {
    ref: ref,
    style: {
      position: 'relative',
      display: 'inline-block',
      minWidth: 160,
      ...style
    }
  }, /*#__PURE__*/React.createElement("button", {
    type: "button",
    disabled: disabled,
    onClick: () => setOpen(o => !o),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      width: '100%',
      height: h,
      padding: '0 12px 0 14px',
      background: 'var(--ink-1000)',
      color: sel ? 'var(--text-primary)' : 'var(--text-tertiary)',
      border: `1px solid ${open ? 'var(--accent)' : 'var(--line)'}`,
      borderRadius: 'var(--radius-md)',
      boxShadow: open ? '0 0 0 3px var(--focus-ring)' : 'none',
      font: 'var(--text-label-font)',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.5 : 1,
      transition: 'var(--t-hover)'
    }
  }, sel && sel.icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: sel.icon,
    size: 16,
    style: {
      color: 'var(--text-secondary)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      textAlign: 'left',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, sel ? sel.label : placeholder), /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "chevronDown",
    size: 16,
    style: {
      color: 'var(--text-tertiary)',
      transform: open ? 'rotate(180deg)' : 'none',
      transition: 'transform var(--dur-fast)'
    }
  })), open && /*#__PURE__*/React.createElement("div", {
    role: "listbox",
    style: {
      position: 'absolute',
      top: 'calc(100% + 6px)',
      [align]: 0,
      minWidth: '100%',
      zIndex: 40,
      background: 'var(--ink-800)',
      border: '1px solid var(--line)',
      borderRadius: 'var(--radius-md)',
      boxShadow: 'var(--shadow-lg)',
      padding: 5,
      maxHeight: 280,
      overflowY: 'auto',
      animation: 'celSelIn var(--dur-fast) var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("style", null, `@keyframes celSelIn{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:none}}`), opts.map(o => {
    const on = o.value === value;
    return /*#__PURE__*/React.createElement("button", {
      key: o.value,
      type: "button",
      role: "option",
      "aria-selected": on,
      onClick: () => {
        onChange && onChange(o.value);
        setOpen(false);
      },
      onMouseEnter: e => {
        if (!on) e.currentTarget.style.background = 'var(--ink-700)';
      },
      onMouseLeave: e => {
        if (!on) e.currentTarget.style.background = 'transparent';
      },
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        width: '100%',
        padding: '8px 10px',
        border: 'none',
        borderRadius: 'var(--radius-sm)',
        cursor: 'pointer',
        textAlign: 'left',
        background: on ? 'var(--teal-tint)' : 'transparent',
        color: on ? 'var(--accent)' : 'var(--text-primary)',
        font: 'var(--text-label-font)'
      }
    }, o.icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: o.icon,
      size: 16
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 1
      }
    }, o.label), on && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: "check",
      size: 15
    }));
  })));
}
Object.assign(__ds_scope, { Select });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Select.jsx", error: String((e && e.message) || e) }); }

// components/core/SettingsGroup.jsx
try { (() => {
/* SettingsGroup + SettingsRow — the standard preferences layout. A group is a
   titled card; rows hold a label + description on the left and a control on the
   right. Used to build any settings/preferences panel consistently. */

function SettingsGroup({
  title,
  description,
  children,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("section", {
    style: {
      marginBottom: 'var(--space-9)',
      ...style
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      marginBottom: 'var(--space-4)'
    }
  }, /*#__PURE__*/React.createElement("h2", {
    style: {
      font: 'var(--text-h3-font)',
      color: 'var(--text-primary)',
      letterSpacing: 'var(--tracking-tight)'
    }
  }, title), description && /*#__PURE__*/React.createElement("p", {
    style: {
      font: 'var(--text-meta-font)',
      color: 'var(--text-tertiary)',
      marginTop: 4
    }
  }, description)), /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--line-soft)',
      borderRadius: 'var(--radius-lg)',
      padding: '4px 20px',
      boxShadow: 'var(--edge-top)'
    }
  }, children));
}
function SettingsRow({
  title,
  description,
  children,
  align = 'center',
  last = false,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: align === 'top' ? 'flex-start' : 'center',
      gap: 24,
      padding: '16px 0',
      borderBottom: last ? 'none' : '1px solid var(--line-soft)',
      ...style
    }
  }, (title || description) && /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--text-title-font)',
      color: 'var(--text-primary)'
    }
  }, title), description && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--text-meta-font)',
      color: 'var(--text-tertiary)',
      marginTop: 3,
      maxWidth: 460
    }
  }, description)), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 'none',
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, children));
}
Object.assign(__ds_scope, { SettingsGroup, SettingsRow });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/SettingsGroup.jsx", error: String((e && e.message) || e) }); }

// components/core/Slider.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/* Slider — seek bar & volume. Teal fill, hidden thumb that appears on hover.
   Controlled: pass value (0..max) and onChange(newValue). buffered draws a
   lighter pre-buffered region behind the fill (for seek bars). */

function Slider({
  value = 0,
  max = 100,
  buffered = null,
  onChange,
  size = 'md',
  showThumb = 'hover',
  disabled = false,
  style = {},
  ...rest
}) {
  const ref = React.useRef(null);
  const [hover, setHover] = React.useState(false);
  const [dragging, setDragging] = React.useState(false);
  const pct = Math.max(0, Math.min(100, value / max * 100));
  const bpct = buffered != null ? Math.max(0, Math.min(100, buffered / max * 100)) : 0;
  const trackH = size === 'sm' ? 4 : size === 'lg' ? 8 : 6;
  const thumb = size === 'lg' ? 16 : 13;
  const showT = showThumb === 'always' || hover || dragging;
  const posToVal = clientX => {
    const r = ref.current.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (clientX - r.left) / r.width));
    return ratio * max;
  };
  const onDown = e => {
    if (disabled) return;
    setDragging(true);
    onChange && onChange(posToVal(e.clientX));
    const move = ev => onChange && onChange(posToVal(ev.clientX));
    const up = () => {
      setDragging(false);
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
  };
  return /*#__PURE__*/React.createElement("div", _extends({
    ref: ref,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    onMouseDown: onDown,
    role: "slider",
    "aria-valuenow": Math.round(value),
    "aria-valuemax": max,
    style: {
      position: 'relative',
      height: thumb,
      display: 'flex',
      alignItems: 'center',
      cursor: disabled ? 'default' : 'pointer',
      width: '100%',
      opacity: disabled ? 0.5 : 1,
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: '100%',
      height: trackH,
      borderRadius: 'var(--radius-pill)',
      background: 'var(--ink-700)',
      overflow: 'hidden'
    }
  }, buffered != null && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      width: bpct + '%',
      background: 'var(--ink-500)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      width: pct + '%',
      background: hover || dragging ? 'var(--accent-hover)' : 'var(--accent)',
      transition: dragging ? 'none' : 'background-color var(--dur-fast)'
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: `calc(${pct}% - ${thumb / 2}px)`,
      width: thumb,
      height: thumb,
      borderRadius: '50%',
      background: '#fff',
      boxShadow: '0 2px 8px rgba(0,0,0,0.5)',
      opacity: showT ? 1 : 0,
      transform: showT ? 'scale(1)' : 'scale(0.5)',
      transition: dragging ? 'none' : 'opacity var(--dur-fast), transform var(--dur-fast)',
      pointerEvents: 'none'
    }
  }));
}
Object.assign(__ds_scope, { Slider });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Slider.jsx", error: String((e && e.message) || e) }); }

// components/core/Switch.jsx
try { (() => {
/* Switch — settings toggle. Teal when on. macOS-ish proportions. */

function Switch({
  checked = false,
  onChange,
  disabled = false,
  size = 'md',
  style = {}
}) {
  const w = size === 'sm' ? 36 : 44;
  const h = size === 'sm' ? 20 : 26;
  const knob = h - 6;
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    role: "switch",
    "aria-checked": checked,
    disabled: disabled,
    onClick: () => !disabled && onChange && onChange(!checked),
    style: {
      width: w,
      height: h,
      flex: 'none',
      borderRadius: 'var(--radius-pill)',
      border: 'none',
      background: checked ? 'var(--accent)' : 'var(--ink-600)',
      position: 'relative',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.5 : 1,
      padding: 0,
      transition: 'background-color var(--dur-base) var(--ease-standard)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: 3,
      left: checked ? w - knob - 3 : 3,
      width: knob,
      height: knob,
      borderRadius: '50%',
      background: '#fff',
      boxShadow: '0 1px 3px rgba(0,0,0,0.4)',
      transition: 'left var(--dur-base) var(--ease-spring)'
    }
  }));
}
Object.assign(__ds_scope, { Switch });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Switch.jsx", error: String((e && e.message) || e) }); }

// components/core/Tabs.jsx
try { (() => {
/* Tabs — underline navigation for detail-page sections (Overview / Tracks /
   Similar). Active tab carries an accent underline. */

function Tabs({
  tabs = [],
  value,
  onChange,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 'var(--space-8)',
      borderBottom: '1px solid var(--line)',
      ...style
    }
  }, tabs.map(t => {
    const val = t.value ?? t;
    const label = t.label ?? t;
    const on = val === value;
    return /*#__PURE__*/React.createElement("button", {
      key: val,
      type: "button",
      onClick: () => onChange && onChange(val),
      style: {
        position: 'relative',
        border: 'none',
        background: 'transparent',
        cursor: 'pointer',
        padding: '0 0 14px',
        font: 'var(--text-title-font)',
        color: on ? 'var(--text-primary)' : 'var(--text-tertiary)',
        transition: 'color var(--dur-fast)'
      }
    }, label, /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: -1,
        height: 2,
        borderRadius: 2,
        background: 'var(--accent)',
        transform: on ? 'scaleX(1)' : 'scaleX(0)',
        transformOrigin: 'left',
        transition: 'transform var(--dur-base) var(--ease-out)'
      }
    }));
  }));
}
Object.assign(__ds_scope, { Tabs });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Tabs.jsx", error: String((e && e.message) || e) }); }

// components/media/CoverArt.jsx
try { (() => {
/* CoverArt — deterministic generative album art from a seed string.
   Used everywhere a real cover would be (tiles, rows, now-playing). If a real
   `src` is supplied it wins; otherwise we synthesize a tasteful gradient +
   motif so the system never ships placeholder-gray or copyrighted art. */

function seedRand(str = '') {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h += 0x6D2B79F5;
    let t = h;
    t = Math.imul(t ^ t >>> 15, t | 1);
    t ^= t + Math.imul(t ^ t >>> 7, t | 61);
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  };
}
const PALETTES = [['#0b3b3a', '#00d9c0', '#03201f'], ['#2a1b4d', '#7c5cff', '#120b29'], ['#4d1b2e', '#ff5470', '#1f0a12'], ['#4a3410', '#e6a23c', '#1c1304'], ['#0d2a4d', '#3b82f6', '#06121f'], ['#143d24', '#34d399', '#06190f'], ['#3d1438', '#e056b8', '#190618'], ['#1a1a22', '#8a93a6', '#070709'], ['#3a2410', '#ff8c42', '#160c04'], ['#10303d', '#22d3ee', '#04141a']];
function CoverArt({
  seed = '',
  src,
  size = 160,
  radius = 'var(--radius-md)',
  shadow = false,
  style = {},
  children
}) {
  const rnd = React.useMemo(() => seedRand(seed), [seed]);
  const {
    bg,
    motif
  } = React.useMemo(() => {
    const pal = PALETTES[Math.floor(rnd() * PALETTES.length)];
    const angle = Math.floor(rnd() * 360);
    const cx = 20 + rnd() * 60,
      cy = 20 + rnd() * 60;
    const kind = Math.floor(rnd() * 3);
    const bg = `radial-gradient(120% 120% at ${cx}% ${cy}%, ${pal[1]} 0%, ${pal[0]} 45%, ${pal[2]} 100%)`;
    let motif = '';
    if (kind === 0) motif = `linear-gradient(${angle}deg, transparent 0 46%, rgba(255,255,255,0.10) 46% 50%, transparent 50%)`;else if (kind === 1) motif = `repeating-linear-gradient(${angle}deg, rgba(255,255,255,0.06) 0 2px, transparent 2px 14px)`;else motif = `radial-gradient(40% 40% at ${100 - cx}% ${100 - cy}%, rgba(0,0,0,0.35) 0%, transparent 70%)`;
    return {
      bg,
      motif
    };
  }, [seed]);
  const dim = typeof size === 'number' ? size + 'px' : size;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      width: dim,
      height: dim,
      borderRadius: radius,
      position: 'relative',
      overflow: 'hidden',
      flex: 'none',
      background: src ? 'var(--ink-700)' : bg,
      boxShadow: shadow ? 'var(--shadow-art)' : 'none',
      ...style
    }
  }, src ? /*#__PURE__*/React.createElement("img", {
    src: src,
    alt: "",
    style: {
      width: '100%',
      height: '100%',
      objectFit: 'cover'
    }
  }) : /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      background: motif,
      mixBlendMode: 'overlay'
    }
  }), children);
}
Object.assign(__ds_scope, { CoverArt });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/media/CoverArt.jsx", error: String((e && e.message) || e) }); }

// components/media/AlbumTile.jsx
try { (() => {
/* AlbumTile — the shelf card. Cover with a hover-revealed play FAB that lifts,
   title + subtitle beneath. `round` variant (for artists) uses a circle cover. */

function AlbumTile({
  title,
  subtitle,
  seed,
  src,
  size = 'var(--tile-lg)',
  round = false,
  onPlay,
  onOpen,
  style = {}
}) {
  const [hover, setHover] = React.useState(false);
  const radius = round ? '50%' : 'var(--radius-lg)';
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onOpen,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 'var(--space-4)',
      width: size,
      padding: 'var(--space-4)',
      border: 'none',
      textAlign: 'left',
      background: hover ? 'var(--ink-800)' : 'transparent',
      borderRadius: 'var(--radius-lg)',
      cursor: 'pointer',
      transition: 'background-color var(--dur-base) var(--ease-standard)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: '100%',
      aspectRatio: '1'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.CoverArt, {
    seed: seed || title,
    src: src,
    size: "100%",
    radius: radius,
    shadow: hover,
    style: {
      aspectRatio: '1',
      boxShadow: hover ? 'var(--shadow-art)' : 'none',
      transition: 'box-shadow var(--dur-base)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      right: 12,
      bottom: 12,
      opacity: hover ? 1 : 0,
      transform: hover ? 'translateY(0)' : 'translateY(8px)',
      transition: 'opacity var(--dur-base) var(--ease-out), transform var(--dur-base) var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.PlayButton, {
    size: "md",
    onToggle: e => {
      e.stopPropagation();
      onPlay && onPlay();
    }
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 3,
      padding: round ? '0 4px' : '0 2px',
      alignItems: round ? 'center' : 'stretch',
      textAlign: round ? 'center' : 'left'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-title-font)',
      color: 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, title), subtitle && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-meta-font)',
      color: 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, subtitle)));
}
Object.assign(__ds_scope, { AlbumTile });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/media/AlbumTile.jsx", error: String((e && e.message) || e) }); }

// components/media/TrackRow.jsx
try { (() => {
/* TrackRow — the core list primitive. Index turns into a play button on hover;
   active row tints teal and shows an equalizer. Like + more on hover/right. */

function EqualizerBars() {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-end',
      gap: 2,
      height: 16,
      width: 16
    },
    "aria-label": "Now playing"
  }, [0, 1, 2, 3].map(i => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      width: 2.5,
      background: 'var(--accent)',
      borderRadius: 1,
      animation: `celeq 0.9s ease-in-out ${i * 0.15}s infinite alternate`,
      height: ['40%', '90%', '60%', '100%'][i]
    }
  })), /*#__PURE__*/React.createElement("style", null, `@keyframes celeq{from{transform:scaleY(0.35)}to{transform:scaleY(1)}}`));
}
function TrackRow({
  index,
  title,
  artist,
  album,
  duration = '0:00',
  explicit = false,
  liked = false,
  active = false,
  playing = false,
  showArt = false,
  seed,
  plays,
  equalizer = true,
  onPlay,
  onLike,
  onMore,
  style = {}
}) {
  const [hover, setHover] = React.useState(false);
  return /*#__PURE__*/React.createElement("div", {
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    onDoubleClick: onPlay,
    style: {
      display: 'grid',
      gridTemplateColumns: `28px ${showArt ? '44px ' : ''}minmax(0,1fr) ${album ? 'minmax(0,0.8fr) ' : ''}${plays ? '90px ' : ''}40px 56px 40px`,
      alignItems: 'center',
      gap: 'var(--space-5)',
      height: 'var(--row-h)',
      padding: '0 var(--space-4)',
      borderRadius: 'var(--radius-md)',
      background: active ? 'var(--teal-tint)' : hover ? 'var(--ink-800)' : 'transparent',
      transition: 'background-color var(--dur-fast)',
      cursor: 'default',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      placeItems: 'center',
      width: 28
    }
  }, active && playing && equalizer ? /*#__PURE__*/React.createElement(EqualizerBars, null) : active && playing ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "volume",
    size: 15,
    style: {
      color: 'var(--accent)'
    }
  }) : hover ? /*#__PURE__*/React.createElement("button", {
    onClick: onPlay,
    "aria-label": "Play",
    style: {
      border: 'none',
      background: 'transparent',
      color: 'var(--text-primary)',
      cursor: 'pointer',
      display: 'grid',
      placeItems: 'center'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "play",
    size: 15
  })) : /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-meta-font)',
      fontFamily: 'var(--font-mono)',
      color: active ? 'var(--accent)' : 'var(--text-tertiary)'
    }
  }, index)), showArt && /*#__PURE__*/React.createElement(__ds_scope.CoverArt, {
    seed: seed || title,
    size: 44,
    radius: "var(--radius-sm)"
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-title-font)',
      color: active ? 'var(--accent)' : 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, title), explicit && /*#__PURE__*/React.createElement(__ds_scope.ExplicitBadge, {
    size: 15
  })), artist && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--text-meta-font)',
      color: 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
      marginTop: 1
    }
  }, artist)), album && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-body-font)',
      color: 'var(--text-tertiary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, album), plays && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-meta-font)',
      fontFamily: 'var(--font-mono)',
      color: 'var(--text-tertiary)',
      textAlign: 'right'
    }
  }, plays), /*#__PURE__*/React.createElement("button", {
    onClick: onLike,
    "aria-label": "Like",
    style: {
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      display: 'grid',
      placeItems: 'center',
      color: liked ? 'var(--accent)' : 'var(--text-tertiary)',
      opacity: liked || hover ? 1 : 0,
      transition: 'opacity var(--dur-fast), color var(--dur-fast)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: liked ? 'heartFill' : 'heart',
    size: 17
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--text-meta-font)',
      fontFamily: 'var(--font-mono)',
      color: 'var(--text-tertiary)',
      textAlign: 'right'
    }
  }, duration), /*#__PURE__*/React.createElement("button", {
    onClick: onMore,
    "aria-label": "More",
    style: {
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      display: 'grid',
      placeItems: 'center',
      color: 'var(--text-tertiary)',
      opacity: hover ? 1 : 0,
      transition: 'opacity var(--dur-fast)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "more",
    size: 18
  })));
}
Object.assign(__ds_scope, { TrackRow });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/media/TrackRow.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/AppShell.jsx
try { (() => {
/* AppShell — Celimos macOS chrome: TitleBar, Sidebar, PlayBar.
   Composes Celimos DS primitives from the global namespace. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    IconButton,
    PlayButton,
    Slider,
    Input,
    Avatar,
    Badge,
    CoverArt,
    Icon
  } = NS;
  const D = window.CELIMOS_DATA;

  /* ---- TitleBar: traffic lights + nav + global search + account ---- */
  function TitleBar({
    ctx
  }) {
    return /*#__PURE__*/React.createElement("header", {
      style: {
        height: 60,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '0 16px 0 14px',
        WebkitAppRegion: 'drag',
        background: 'var(--bg-chrome)',
        borderBottom: '1px solid var(--line-soft)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 8,
        alignItems: 'center'
      }
    }, ['#ff5f57', '#febc2e', '#28c840'].map(c => /*#__PURE__*/React.createElement("span", {
      key: c,
      style: {
        width: 12,
        height: 12,
        borderRadius: '50%',
        background: c
      }
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 2,
        marginLeft: 6
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      name: "chevronLeft",
      size: "sm",
      title: "Back",
      onClick: ctx.back
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "chevronRight",
      size: "sm",
      title: "Forward"
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        width: 380,
        maxWidth: '40%'
      }
    }, /*#__PURE__*/React.createElement(Input, {
      icon: "search",
      size: "md",
      placeholder: "Search artists, albums, tracks",
      value: ctx.view.type === 'search' ? ctx.query : '',
      onChange: ctx.search,
      onFocus: () => ctx.view.type !== 'search' && ctx.go({
        type: 'search'
      })
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }), /*#__PURE__*/React.createElement(Badge, {
      tone: "outline",
      style: {
        gap: 6
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 7,
        height: 7,
        borderRadius: '50%',
        background: 'var(--success)',
        boxShadow: '0 0 8px var(--success)'
      }
    }), "Lossless"), /*#__PURE__*/React.createElement("div", {
      onClick: () => ctx.go({
        type: 'settings'
      }),
      style: {
        cursor: 'pointer'
      }
    }, /*#__PURE__*/React.createElement(Avatar, {
      name: "Alex Rivera",
      size: 30
    })));
  }

  /* ---- Sidebar: logo, nav, playlists, server status ---- */
  function NavItem({
    icon,
    label,
    active,
    onClick
  }) {
    const [h, setH] = React.useState(false);
    return /*#__PURE__*/React.createElement("button", {
      onClick: onClick,
      onMouseEnter: () => setH(true),
      onMouseLeave: () => setH(false),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        width: '100%',
        height: 42,
        padding: '0 14px',
        border: 'none',
        borderRadius: 'var(--radius-md)',
        cursor: 'pointer',
        background: active ? 'var(--ink-750)' : h ? 'var(--ink-850)' : 'transparent',
        color: active ? 'var(--text-primary)' : h ? 'var(--text-primary)' : 'var(--text-secondary)',
        font: 'var(--text-title-font)',
        textAlign: 'left',
        transition: 'var(--t-hover)',
        position: 'relative'
      }
    }, active && /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: 0,
        top: 11,
        bottom: 11,
        width: 3,
        borderRadius: 3,
        background: 'var(--accent)'
      }
    }), /*#__PURE__*/React.createElement(Icon, {
      name: icon,
      size: 21
    }), label);
  }
  function Sidebar({
    ctx
  }) {
    const v = ctx.view.type;
    return /*#__PURE__*/React.createElement("nav", {
      style: {
        width: 'var(--sidebar-w)',
        flex: 'none',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--bg-chrome)',
        borderRight: '1px solid var(--line-soft)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '20px 20px 14px'
      }
    }, /*#__PURE__*/React.createElement("img", {
      src: "../../assets/logo-mark.svg",
      width: "26",
      height: "26",
      alt: ""
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--weight-display) 21px/1 var(--font-display)',
        letterSpacing: '-0.02em',
        color: 'var(--text-primary)'
      }
    }, "Celimos")), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 3,
        padding: '0 12px'
      }
    }, /*#__PURE__*/React.createElement(NavItem, {
      icon: "home",
      label: "Home",
      active: v === 'home',
      onClick: () => ctx.go({
        type: 'home'
      })
    }), /*#__PURE__*/React.createElement(NavItem, {
      icon: "search",
      label: "Search",
      active: v === 'search',
      onClick: () => ctx.go({
        type: 'search'
      })
    }), /*#__PURE__*/React.createElement(NavItem, {
      icon: "library",
      label: "Library",
      active: v === 'library',
      onClick: () => ctx.go({
        type: 'library'
      })
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '22px 22px 10px'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--weight-semibold) 11px/1 var(--font-ui)',
        letterSpacing: '0.14em',
        textTransform: 'uppercase',
        color: 'var(--text-tertiary)'
      }
    }, "Playlists"), /*#__PURE__*/React.createElement(IconButton, {
      name: "plus",
      size: "sm",
      title: "New playlist"
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        overflowY: 'auto',
        padding: '0 12px',
        display: 'flex',
        flexDirection: 'column',
        gap: 2
      }
    }, D.playlists.map(p => {
      const active = ctx.view.type === 'playlist' && ctx.view.id === p.id;
      return /*#__PURE__*/React.createElement("button", {
        key: p.id,
        onClick: () => ctx.go({
          type: 'playlist',
          id: p.id
        }),
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: 11,
          width: '100%',
          padding: '7px 8px',
          border: 'none',
          borderRadius: 'var(--radius-md)',
          cursor: 'pointer',
          textAlign: 'left',
          background: active ? 'var(--ink-800)' : 'transparent',
          transition: 'var(--t-hover)'
        },
        onMouseEnter: e => {
          if (!active) e.currentTarget.style.background = 'var(--ink-850)';
        },
        onMouseLeave: e => {
          if (!active) e.currentTarget.style.background = 'transparent';
        }
      }, /*#__PURE__*/React.createElement(CoverArt, {
        seed: p.seed,
        size: 38,
        radius: "var(--radius-sm)"
      }), /*#__PURE__*/React.createElement("span", {
        style: {
          minWidth: 0
        }
      }, /*#__PURE__*/React.createElement("span", {
        style: {
          display: 'block',
          font: 'var(--text-label-font)',
          color: 'var(--text-primary)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap'
        }
      }, p.title), /*#__PURE__*/React.createElement("span", {
        style: {
          display: 'block',
          font: 'var(--text-meta-font)',
          color: 'var(--text-tertiary)'
        }
      }, "Playlist \xB7 ", p.by)));
    })), /*#__PURE__*/React.createElement("button", {
      onClick: () => ctx.go({
        type: 'settings'
      }),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        margin: 12,
        padding: '10px 12px',
        border: '1px solid var(--line-soft)',
        borderRadius: 'var(--radius-md)',
        cursor: 'pointer',
        background: 'var(--ink-850)',
        textAlign: 'left'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "server",
      size: 18,
      style: {
        color: 'var(--success)'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        minWidth: 0,
        flex: 1
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        font: 'var(--text-meta-font)',
        color: 'var(--text-secondary)'
      }
    }, "Connected"), /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        font: 'var(--text-meta-font)',
        fontFamily: 'var(--font-mono)',
        color: 'var(--text-tertiary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, "navidrome.local")), /*#__PURE__*/React.createElement(Icon, {
      name: "settings",
      size: 16,
      style: {
        color: 'var(--text-tertiary)'
      }
    })));
  }

  /* ---- PlayBar: now-playing strip docked at the bottom ---- */
  function fmt(sec) {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return m + ':' + String(s).padStart(2, '0');
  }
  function PlayBar({
    ctx
  }) {
    const t = ctx.current;
    const dur = ctx.duration || 222;
    return /*#__PURE__*/React.createElement("footer", {
      style: {
        height: 'var(--playbar-h)',
        flex: 'none',
        display: 'grid',
        gridTemplateColumns: '1fr minmax(420px, 1.6fr) 1fr',
        alignItems: 'center',
        gap: 24,
        padding: '0 20px',
        background: 'var(--bg-chrome)',
        borderTop: '1px solid var(--line)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 13,
        minWidth: 0
      }
    }, t && /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(CoverArt, {
      seed: t.seed,
      size: 54,
      radius: "var(--radius-sm)",
      shadow: true
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--text-title-font)',
        color: 'var(--text-primary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, t.title), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--text-meta-font)',
        color: 'var(--text-secondary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, t.artist)), /*#__PURE__*/React.createElement(IconButton, {
      name: ctx.isLiked(t) ? 'heartFill' : 'heart',
      size: "sm",
      active: ctx.isLiked(t),
      onClick: () => ctx.toggleLike(t),
      title: "Like"
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 7,
        width: '100%'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      name: "shuffle",
      size: "sm",
      active: ctx.shuffle,
      onClick: ctx.toggleShuffle,
      title: "Shuffle"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "skipBack",
      onClick: ctx.prev,
      title: "Previous"
    }), /*#__PURE__*/React.createElement(PlayButton, {
      size: "md",
      playing: ctx.playing,
      onToggle: ctx.togglePlay
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "skipForward",
      onClick: ctx.next,
      title: "Next"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: ctx.repeat === 'one' ? 'repeatOne' : 'repeat',
      size: "sm",
      active: ctx.repeat !== 'off',
      onClick: ctx.cycleRepeat,
      title: "Repeat"
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        width: '100%'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-meta-font)',
        fontFamily: 'var(--font-mono)',
        color: 'var(--text-tertiary)',
        width: 38,
        textAlign: 'right'
      }
    }, fmt(ctx.progress)), /*#__PURE__*/React.createElement(Slider, {
      value: ctx.progress,
      max: dur,
      buffered: Math.min(dur, ctx.progress + 40),
      onChange: ctx.seek
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-meta-font)',
        fontFamily: 'var(--font-mono)',
        color: 'var(--text-tertiary)',
        width: 38
      }
    }, fmt(dur)))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-end',
        gap: 4
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      name: "lyrics",
      size: "sm",
      title: "Lyrics"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "queue",
      size: "sm",
      active: ctx.queueOpen,
      onClick: ctx.toggleQueue,
      title: "Queue"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "airplay",
      size: "sm",
      title: "AirPlay"
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        width: 120
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      name: ctx.volume === 0 ? 'volumeMute' : ctx.volume < 50 ? 'volumeLow' : 'volume',
      size: "sm",
      onClick: ctx.toggleMute
    }), /*#__PURE__*/React.createElement(Slider, {
      value: ctx.volume,
      onChange: ctx.setVolume,
      size: "sm"
    })), /*#__PURE__*/React.createElement(IconButton, {
      name: "chevronDown",
      size: "sm",
      title: "Expand player",
      active: true,
      onClick: ctx.openNowPlaying,
      style: {
        transform: 'rotate(180deg)'
      }
    })));
  }
  window.CelimosShell = {
    TitleBar,
    Sidebar,
    PlayBar,
    fmt
  };
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/AppShell.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/ArtistScreen.jsx
try { (() => {
/* ArtistScreen — full-bleed hero, popular tracks, discography. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    CoverArt,
    PlayButton,
    IconButton,
    Button,
    TrackRow,
    AlbumTile,
    Icon
  } = NS;
  const {
    Page,
    Shelf
  } = window.CelimosCommon;
  const D = window.CELIMOS_DATA;
  function hashHue(s = '') {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
    return h;
  }
  function ArtistScreen({
    ctx
  }) {
    const artist = D.artistList.find(a => a.name === ctx.view.id);
    if (!artist) return null;
    const albums = D.albumList.filter(a => a.artist.name === artist.name);
    const fallback = albums.length ? albums : D.albumList.slice(0, 2);
    const [following, setFollowing] = React.useState(false);

    // top tracks = first tracks of this artist's albums (or sample)
    const top = fallback.flatMap(al => al.tracks.slice(0, 2).map(t => ({
      ...t,
      album: al.title,
      seed: al.seed,
      artist: artist.name
    }))).slice(0, 5).map((t, i) => ({
      ...t,
      index: i + 1,
      plays: (1.8 - i * 0.3).toFixed(1) + 'M'
    }));
    const hue = hashHue(artist.seed);
    return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'relative',
        height: 380
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: artist.seed,
      size: "100%",
      radius: "0",
      style: {
        position: 'absolute',
        inset: 0,
        height: 380
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: 'linear-gradient(180deg, rgba(6,6,7,0.1) 0%, rgba(6,6,7,0.45) 55%, var(--bg-window) 100%)'
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        left: 'var(--content-pad)',
        bottom: 28,
        right: 'var(--content-pad)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        marginBottom: 14
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 22,
        height: 22,
        borderRadius: '50%',
        background: 'var(--info)',
        display: 'grid',
        placeItems: 'center'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "check",
      size: 13,
      style: {
        color: '#04101f'
      }
    })), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-label-font)',
        color: 'var(--on-art)'
      }
    }, "Verified Artist")), /*#__PURE__*/React.createElement("h1", {
      style: {
        font: 'var(--weight-display) 72px/0.95 var(--font-display)',
        letterSpacing: '-0.03em',
        color: 'var(--on-art)',
        marginBottom: 14,
        textShadow: '0 2px 30px rgba(0,0,0,0.5)'
      }
    }, artist.name), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-body-font)',
        color: 'var(--on-art)',
        opacity: 0.85
      }
    }, "1,284,902 monthly listeners \xB7 ", artist.genre))), /*#__PURE__*/React.createElement(Page, {
      style: {
        paddingTop: 22
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-5)',
        marginBottom: 'var(--space-9)'
      }
    }, /*#__PURE__*/React.createElement(PlayButton, {
      size: "xl",
      playing: ctx.playing && ctx.contextId === artist.name,
      onToggle: () => ctx.playArtist(artist)
    }), /*#__PURE__*/React.createElement(Button, {
      variant: following ? 'secondary' : 'outline',
      size: "lg",
      onClick: () => setFollowing(f => !f),
      iconLeft: following ? 'check' : 'plus'
    }, following ? 'Following' : 'Follow'), /*#__PURE__*/React.createElement(IconButton, {
      name: "more",
      size: "lg",
      title: "More"
    })), /*#__PURE__*/React.createElement("h2", {
      style: {
        font: 'var(--text-h2-font)',
        letterSpacing: 'var(--tracking-tight)',
        marginBottom: 'var(--space-5)'
      }
    }, "Popular"), /*#__PURE__*/React.createElement("div", {
      style: {
        marginBottom: 'var(--space-10)'
      }
    }, top.map(t => /*#__PURE__*/React.createElement(TrackRow, {
      key: t.index,
      index: t.index,
      title: t.title,
      showArt: true,
      seed: t.seed,
      plays: t.plays,
      duration: t.duration,
      explicit: t.explicit,
      liked: ctx.isLiked(t),
      active: ctx.current && ctx.current.title === t.title,
      playing: ctx.playing && ctx.current && ctx.current.title === t.title,
      onPlay: () => ctx.playTrack(t, artist),
      onLike: () => ctx.toggleLike(t),
      onMore: () => {}
    }))), /*#__PURE__*/React.createElement(Shelf, {
      title: "Discography",
      action: "Show all"
    }, fallback.map(a => /*#__PURE__*/React.createElement(AlbumTile, {
      key: a.id,
      title: a.title,
      subtitle: a.year + ' · Album',
      seed: a.seed,
      size: "var(--tile-lg)",
      onOpen: () => ctx.go({
        type: 'album',
        id: a.id
      }),
      onPlay: () => ctx.playContext(a)
    })))));
  }
  window.ArtistScreen = ArtistScreen;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/ArtistScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/CollectionScreen.jsx
try { (() => {
/* CollectionScreen — shared album / playlist detail: washed header + track list. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    CoverArt,
    PlayButton,
    IconButton,
    Button,
    TrackRow,
    Icon,
    QualityBadge
  } = NS;
  const {
    Page
  } = window.CelimosCommon;
  const D = window.CELIMOS_DATA;
  function hashHue(s = '') {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
    return h;
  }
  function totalDuration(tracks) {
    let sec = tracks.reduce((a, t) => {
      const [m, s] = t.duration.split(':').map(Number);
      return a + m * 60 + s;
    }, 0);
    const h = Math.floor(sec / 3600),
      m = Math.round(sec % 3600 / 60);
    return h ? `${h} hr ${m} min` : `${m} min`;
  }
  function ColumnHeader({
    hasAlbum
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: `28px minmax(0,1fr) ${hasAlbum ? 'minmax(0,0.8fr) ' : ''}40px 56px 40px`,
        alignItems: 'center',
        gap: 'var(--space-5)',
        height: 36,
        padding: '0 var(--space-4)',
        borderBottom: '1px solid var(--line-soft)',
        marginBottom: 8,
        font: 'var(--weight-medium) 11px/1 var(--font-ui)',
        letterSpacing: '0.1em',
        textTransform: 'uppercase',
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        textAlign: 'center'
      }
    }, "#"), /*#__PURE__*/React.createElement("span", null, "Title"), hasAlbum && /*#__PURE__*/React.createElement("span", null, "Album"), /*#__PURE__*/React.createElement("span", null), /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'grid',
        placeItems: 'center'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "clock",
      size: 15
    })), /*#__PURE__*/React.createElement("span", null));
  }
  function CollectionScreen({
    ctx
  }) {
    const isAlbum = ctx.view.type === 'album';
    const col = isAlbum ? D.albums[ctx.view.id] : D.playlists.find(p => p.id === ctx.view.id);
    if (!col) return null;
    const tracks = isAlbum ? col.tracks.map((t, i) => ({
      ...t,
      artist: col.artist.name,
      album: col.title,
      seed: col.seed,
      index: i + 1
    })) : col.tracksFrom.flatMap(aid => D.albums[aid].tracks.slice(0, 3).map(t => ({
      ...t,
      artist: D.albums[aid].artist.name,
      album: D.albums[aid].title,
      seed: D.albums[aid].seed
    }))).map((t, i) => ({
      ...t,
      index: i + 1
    }));
    const hue = hashHue(col.seed);
    const title = col.title;
    const meta = isAlbum ? `${col.artist.name} · ${col.year} · ${tracks.length} songs · ${totalDuration(tracks)}` : `${col.by} · ${tracks.length} songs · ${totalDuration(tracks)}`;
    return /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'relative'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        height: 420,
        pointerEvents: 'none',
        background: `linear-gradient(180deg, hsl(${hue} 42% 26%) 0%, hsl(${hue} 30% 12%) 42%, var(--bg-window) 100%)`
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'relative'
      }
    }, /*#__PURE__*/React.createElement(Page, {
      style: {
        paddingBottom: 28
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 'var(--space-8)',
        alignItems: 'flex-end',
        paddingTop: 12
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: col.seed,
      size: 232,
      radius: "var(--radius-lg)",
      shadow: true
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        minWidth: 0,
        paddingBottom: 6
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--weight-semibold) 12px/1 var(--font-ui)',
        letterSpacing: '0.12em',
        textTransform: 'uppercase',
        color: 'var(--text-secondary)'
      }
    }, isAlbum ? col.type : 'Playlist'), /*#__PURE__*/React.createElement("h1", {
      style: {
        font: 'var(--text-display-font)',
        letterSpacing: 'var(--tracking-display)',
        margin: '12px 0 14px',
        color: 'var(--text-primary)'
      }
    }, title), !isAlbum && /*#__PURE__*/React.createElement("p", {
      style: {
        font: 'var(--text-body-font)',
        color: 'var(--text-secondary)',
        marginBottom: 12,
        maxWidth: 560
      }
    }, col.desc), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        font: 'var(--text-label-font)',
        color: 'var(--text-secondary)'
      }
    }, isAlbum && /*#__PURE__*/React.createElement(QualityBadge, {
      level: "Lossless"
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-ui)'
      }
    }, meta)))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-5)',
        marginTop: 'var(--space-8)'
      }
    }, /*#__PURE__*/React.createElement(PlayButton, {
      size: "xl",
      playing: ctx.playing && ctx.contextId === col.id,
      onToggle: () => ctx.playContext(col)
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "shuffle",
      size: "lg",
      active: ctx.shuffle,
      onClick: ctx.toggleShuffle,
      title: "Shuffle"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "heart",
      size: "lg",
      title: "Save"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "download",
      size: "lg",
      title: "Download"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "more",
      size: "lg",
      title: "More"
    }))), /*#__PURE__*/React.createElement(Page, {
      style: {
        paddingTop: 4
      }
    }, /*#__PURE__*/React.createElement(ColumnHeader, {
      hasAlbum: !isAlbum
    }), tracks.map(t => /*#__PURE__*/React.createElement(TrackRow, {
      key: t.index,
      index: t.index,
      title: t.title,
      artist: isAlbum ? null : t.artist,
      album: isAlbum ? null : t.album,
      duration: t.duration,
      explicit: t.explicit,
      liked: ctx.isLiked(t),
      active: ctx.current && ctx.current.title === t.title,
      playing: ctx.playing && ctx.current && ctx.current.title === t.title,
      equalizer: ctx.prefs ? ctx.prefs.showEqualizer : true,
      onPlay: () => ctx.playTrack(t, col),
      onLike: () => ctx.toggleLike(t),
      onMore: () => {}
    })), isAlbum && /*#__PURE__*/React.createElement("div", {
      style: {
        marginTop: 'var(--space-9)'
      }
    }, /*#__PURE__*/React.createElement("h2", {
      style: {
        font: 'var(--text-h3-font)',
        color: 'var(--text-primary)',
        marginBottom: 6
      }
    }, "More from ", col.artist.name), /*#__PURE__*/React.createElement("button", {
      onClick: () => ctx.go({
        type: 'artist',
        id: col.artist.name
      }),
      style: {
        border: 'none',
        background: 'transparent',
        cursor: 'pointer',
        font: 'var(--text-label-font)',
        color: 'var(--accent)'
      }
    }, "Go to artist \u2192")))));
  }
  window.CollectionScreen = CollectionScreen;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/CollectionScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/Common.jsx
try { (() => {
/* Common screen primitives shared across Celimos screens. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    AlbumTile,
    Icon,
    IconButton
  } = NS;

  /* A scrollable section of tiles with a header + "Show all". */
  function Shelf({
    title,
    action,
    children
  }) {
    return /*#__PURE__*/React.createElement("section", {
      style: {
        marginBottom: 'var(--space-10)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'baseline',
        justifyContent: 'space-between',
        marginBottom: 'var(--space-5)'
      }
    }, /*#__PURE__*/React.createElement("h2", {
      style: {
        font: 'var(--text-h2-font)',
        letterSpacing: 'var(--tracking-tight)',
        color: 'var(--text-primary)'
      }
    }, title), action && /*#__PURE__*/React.createElement("button", {
      style: {
        border: 'none',
        background: 'transparent',
        cursor: 'pointer',
        font: 'var(--text-label-font)',
        color: 'var(--text-tertiary)',
        letterSpacing: 'var(--tracking-wide)',
        textTransform: 'uppercase'
      }
    }, action)), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 'var(--space-3)',
        overflowX: 'auto',
        paddingBottom: 4,
        margin: '0 -8px',
        scrollbarWidth: 'none'
      }
    }, children));
  }
  function TileShelf({
    title,
    items,
    ctx,
    round
  }) {
    return /*#__PURE__*/React.createElement(Shelf, {
      title: title,
      action: "Show all"
    }, items.map(it => /*#__PURE__*/React.createElement(AlbumTile, {
      key: it.id || it.title,
      title: it.title,
      subtitle: it.subtitle,
      seed: it.seed,
      round: round,
      size: "var(--tile-lg)",
      onOpen: it.onOpen,
      onPlay: it.onPlay
    })));
  }

  /* Page scroll container with consistent gutters. */
  function Page({
    children,
    style
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        padding: 'var(--content-pad)',
        paddingBottom: 40,
        ...style
      }
    }, children);
  }
  window.CelimosCommon = {
    Shelf,
    TileShelf,
    Page
  };
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/Common.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/HomeScreen.jsx
try { (() => {
/* HomeScreen — greeting, quick picks, and content shelves. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    CoverArt,
    Icon,
    PlayButton
  } = NS;
  const {
    Shelf,
    TileShelf,
    Page
  } = window.CelimosCommon;
  const D = window.CELIMOS_DATA;
  function greeting() {
    const h = new Date().getHours();
    return h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening';
  }
  function QuickPick({
    item,
    onOpen,
    onPlay
  }) {
    const [h, setH] = React.useState(false);
    return /*#__PURE__*/React.createElement("button", {
      onClick: onOpen,
      onMouseEnter: () => setH(true),
      onMouseLeave: () => setH(false),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        height: 68,
        padding: '0 16px 0 0',
        minWidth: 0,
        background: h ? 'var(--ink-750)' : 'var(--ink-800)',
        border: '1px solid var(--line-soft)',
        borderRadius: 'var(--radius-md)',
        overflow: 'hidden',
        cursor: 'pointer',
        textAlign: 'left',
        transition: 'var(--t-hover)',
        position: 'relative'
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: item.seed,
      size: 68,
      radius: "0"
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-title-font)',
        color: 'var(--text-primary)',
        flex: 1,
        minWidth: 0,
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, item.title), /*#__PURE__*/React.createElement("span", {
      style: {
        opacity: h ? 1 : 0,
        transform: h ? 'translateX(0)' : 'translateX(6px)',
        transition: 'var(--t-hover)',
        marginRight: 4
      }
    }, /*#__PURE__*/React.createElement(PlayButton, {
      size: "sm",
      onToggle: e => {
        e.stopPropagation();
        onPlay();
      }
    })));
  }
  function HomeScreen({
    ctx
  }) {
    const D_ = D;
    const quick = [{
      ...D_.albums.longnight
    }, {
      ...D_.playlists[0]
    }, {
      ...D_.albums.papermoons
    }, {
      ...D_.playlists[2]
    }, {
      ...D_.albums.monolith
    }, {
      ...D_.playlists[3]
    }];
    return /*#__PURE__*/React.createElement(Page, null, /*#__PURE__*/React.createElement("h1", {
      style: {
        font: 'var(--text-h1-font)',
        letterSpacing: 'var(--tracking-tight)',
        marginBottom: 'var(--space-7)'
      }
    }, greeting()), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: 'var(--space-4)',
        marginBottom: 'var(--space-11)'
      }
    }, quick.map((it, i) => /*#__PURE__*/React.createElement(QuickPick, {
      key: i,
      item: it,
      onOpen: () => it.tracks ? ctx.go({
        type: 'album',
        id: it.id
      }) : ctx.go({
        type: 'playlist',
        id: it.id
      }),
      onPlay: () => ctx.playContext(it)
    }))), /*#__PURE__*/React.createElement(TileShelf, {
      title: "Recently added",
      ctx: ctx,
      items: D_.albumList.map(a => ({
        id: a.id,
        title: a.title,
        subtitle: a.artist.name + ' · ' + a.year,
        seed: a.seed,
        onOpen: () => ctx.go({
          type: 'album',
          id: a.id
        }),
        onPlay: () => ctx.playContext(a)
      }))
    }), /*#__PURE__*/React.createElement(TileShelf, {
      title: "Made for you",
      ctx: ctx,
      items: D_.playlists.map(p => ({
        id: p.id,
        title: p.title,
        subtitle: p.desc,
        seed: p.seed,
        onOpen: () => ctx.go({
          type: 'playlist',
          id: p.id
        }),
        onPlay: () => ctx.playContext(p)
      }))
    }), /*#__PURE__*/React.createElement(TileShelf, {
      title: "Artists you follow",
      ctx: ctx,
      round: true,
      items: D_.artistList.map(a => ({
        id: a.name,
        title: a.name,
        subtitle: a.genre,
        seed: a.seed,
        onOpen: () => ctx.go({
          type: 'artist',
          id: a.name
        }),
        onPlay: () => ctx.playArtist(a)
      }))
    }));
  }
  window.HomeScreen = HomeScreen;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/HomeScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/LibraryScreen.jsx
try { (() => {
/* LibraryScreen — segmented Albums / Artists / Tracks / Playlists. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    SegmentedControl,
    AlbumTile,
    TrackRow,
    IconButton,
    Icon
  } = NS;
  const {
    Page
  } = window.CelimosCommon;
  const D = window.CELIMOS_DATA;
  function LibraryScreen({
    ctx
  }) {
    const [tab, setTab] = React.useState('albums');
    const [view, setView] = React.useState(ctx.prefs.libraryView || 'grid');
    React.useEffect(() => {
      setView(ctx.prefs.libraryView || 'grid');
    }, [ctx.prefs.libraryView]);
    const round = ctx.prefs.roundArtistArt !== false;
    const grid = items => /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: `repeat(auto-fill, minmax(${ctx.prefs.cardSize || 176}px, 1fr))`,
        gap: 'var(--space-3)'
      }
    }, items);
    const rows = items => /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column'
      }
    }, items);
    let body;
    if (tab === 'albums') {
      body = view === 'list' ? rows(D.albumList.map((a, i) => /*#__PURE__*/React.createElement(NS.TrackRow, {
        key: a.id,
        index: i + 1,
        title: a.title,
        artist: a.artist.name + ' · ' + a.year,
        showArt: true,
        seed: a.seed,
        duration: a.tracks.length + ' songs',
        onPlay: () => ctx.go({
          type: 'album',
          id: a.id
        }),
        onMore: () => {}
      }))) : grid(D.albumList.map(a => /*#__PURE__*/React.createElement(AlbumTile, {
        key: a.id,
        title: a.title,
        subtitle: a.artist.name + ' · ' + a.year,
        seed: a.seed,
        size: "100%",
        onOpen: () => ctx.go({
          type: 'album',
          id: a.id
        }),
        onPlay: () => ctx.playContext(a)
      })));
    } else if (tab === 'artists') {
      body = grid(D.artistList.map(a => /*#__PURE__*/React.createElement(AlbumTile, {
        key: a.name,
        title: a.name,
        subtitle: "Artist",
        seed: a.seed,
        round: round,
        size: "100%",
        onOpen: () => ctx.go({
          type: 'artist',
          id: a.name
        }),
        onPlay: () => ctx.playArtist(a)
      })));
    } else if (tab === 'playlists') {
      body = grid(D.playlists.map(p => /*#__PURE__*/React.createElement(AlbumTile, {
        key: p.id,
        title: p.title,
        subtitle: 'Playlist · ' + p.by,
        seed: p.seed,
        size: "100%",
        onOpen: () => ctx.go({
          type: 'playlist',
          id: p.id
        }),
        onPlay: () => ctx.playContext(p)
      })));
    } else {
      const tracks = D.albumList.flatMap(a => a.tracks.filter(t => t.liked).map(t => ({
        ...t,
        album: a.title,
        artist: a.artist.name,
        seed: a.seed
      }))).map((t, i) => ({
        ...t,
        index: i + 1
      }));
      body = /*#__PURE__*/React.createElement("div", null, tracks.map(t => /*#__PURE__*/React.createElement(TrackRow, {
        key: t.index,
        index: t.index,
        title: t.title,
        artist: t.artist,
        album: t.album,
        showArt: true,
        seed: t.seed,
        duration: t.duration,
        explicit: t.explicit,
        liked: true,
        onPlay: () => ctx.playTrack(t),
        onLike: () => ctx.toggleLike(t),
        onMore: () => {}
      })));
    }
    return /*#__PURE__*/React.createElement(Page, null, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        marginBottom: 'var(--space-7)'
      }
    }, /*#__PURE__*/React.createElement("h1", {
      style: {
        font: 'var(--text-h1-font)',
        letterSpacing: 'var(--tracking-tight)'
      }
    }, "Your Library"), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      name: "sort",
      title: "Sort"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: view === 'grid' ? 'rows' : 'grid',
      title: "View",
      onClick: () => setView(v => v === 'grid' ? 'list' : 'grid')
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        marginBottom: 'var(--space-8)'
      }
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Albums',
        value: 'albums'
      }, {
        label: 'Artists',
        value: 'artists'
      }, {
        label: 'Playlists',
        value: 'playlists'
      }, {
        label: 'Liked tracks',
        value: 'tracks'
      }],
      value: tab,
      onChange: setTab
    })), body);
  }
  window.LibraryScreen = LibraryScreen;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/LibraryScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/NowPlaying.jsx
try { (() => {
/* NowPlaying — fullscreen player overlay (the "expand" view). */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    CoverArt,
    PlayButton,
    IconButton,
    Slider,
    QualityBadge,
    Icon
  } = NS;
  const {
    fmt
  } = window.CelimosShell;
  const D = window.CELIMOS_DATA;
  function hashHue(s = '') {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
    return h;
  }
  function NowPlaying({
    ctx
  }) {
    const t = ctx.current;
    if (!t) return null;
    const dur = ctx.duration || 222;
    const hue = hashHue(t.seed || t.title);
    const [tab, setTab] = React.useState('queue');
    const solidBg = ctx.prefs && ctx.prefs.nowPlayingBg === 'solid';
    const bg = solidBg ? 'var(--ink-1000)' : `radial-gradient(120% 80% at 25% 10%, hsl(${hue} 45% 22%) 0%, hsl(${hue} 35% 9%) 45%, var(--ink-1000) 100%)`;
    const upNext = ctx.queue.slice(0, 12);
    return /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        inset: 0,
        zIndex: 50,
        display: 'flex',
        flexDirection: 'column',
        background: bg,
        animation: 'npIn var(--dur-slow) var(--ease-out)'
      }
    }, /*#__PURE__*/React.createElement("style", null, `@keyframes npIn{from{transform:translateY(24px);opacity:0}to{transform:translateY(0);opacity:1}}`), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '20px 28px',
        flex: 'none'
      }
    }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--weight-semibold) 11px/1 var(--font-ui)',
        letterSpacing: '0.14em',
        textTransform: 'uppercase',
        color: 'var(--on-art)',
        opacity: 0.7
      }
    }, "Playing from ", ctx.contextLabel || 'album'), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--text-title-font)',
        color: 'var(--on-art)',
        marginTop: 4
      }
    }, t.album)), /*#__PURE__*/React.createElement(IconButton, {
      name: "chevronDown",
      size: "lg",
      title: "Close",
      onClick: ctx.closeNowPlaying
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        display: 'grid',
        gridTemplateColumns: '1.3fr 1fr',
        gap: 48,
        padding: '0 48px 12px',
        minHeight: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        alignItems: 'center',
        gap: 32,
        minHeight: 0
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: t.seed || t.title,
      size: "min(46vh, 440px)",
      radius: "var(--radius-xl)",
      shadow: true,
      style: {
        aspectRatio: '1',
        boxShadow: 'var(--shadow-xl)'
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        width: '100%',
        maxWidth: 440
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'space-between',
        gap: 16
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10
      }
    }, /*#__PURE__*/React.createElement("h1", {
      style: {
        font: 'var(--weight-bold) 30px/1.1 var(--font-display)',
        letterSpacing: '-0.02em',
        color: 'var(--on-art)'
      }
    }, t.title), /*#__PURE__*/React.createElement(QualityBadge, {
      level: "Lossless"
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--text-h3-font)',
        color: 'var(--on-art)',
        opacity: 0.7,
        marginTop: 6
      }
    }, t.artist)), /*#__PURE__*/React.createElement(IconButton, {
      name: ctx.isLiked(t) ? 'heartFill' : 'heart',
      size: "lg",
      active: ctx.isLiked(t),
      onClick: () => ctx.toggleLike(t)
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        marginTop: 22
      }
    }, /*#__PURE__*/React.createElement(Slider, {
      value: ctx.progress,
      max: dur,
      buffered: Math.min(dur, ctx.progress + 40),
      onChange: ctx.seek,
      size: "lg"
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        justifyContent: 'space-between',
        marginTop: 8,
        font: 'var(--text-meta-font)',
        fontFamily: 'var(--font-mono)',
        color: 'var(--on-art)',
        opacity: 0.7
      }
    }, /*#__PURE__*/React.createElement("span", null, fmt(ctx.progress)), /*#__PURE__*/React.createElement("span", null, "-", fmt(dur - ctx.progress)))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 18,
        marginTop: 16
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      name: "shuffle",
      active: ctx.shuffle,
      onClick: ctx.toggleShuffle,
      title: "Shuffle"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "skipBack",
      size: "lg",
      onClick: ctx.prev,
      title: "Previous"
    }), /*#__PURE__*/React.createElement(PlayButton, {
      size: "xl",
      tone: "light",
      playing: ctx.playing,
      onToggle: ctx.togglePlay
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: "skipForward",
      size: "lg",
      onClick: ctx.next,
      title: "Next"
    }), /*#__PURE__*/React.createElement(IconButton, {
      name: ctx.repeat === 'one' ? 'repeatOne' : 'repeat',
      active: ctx.repeat !== 'off',
      onClick: ctx.cycleRepeat,
      title: "Repeat"
    })))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        minHeight: 0,
        paddingBottom: 20
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 24,
        marginBottom: 16,
        flex: 'none'
      }
    }, ['queue', 'lyrics'].map(x => /*#__PURE__*/React.createElement("button", {
      key: x,
      onClick: () => setTab(x),
      style: {
        border: 'none',
        background: 'transparent',
        cursor: 'pointer',
        font: 'var(--text-title-font)',
        textTransform: 'capitalize',
        padding: '0 0 8px',
        color: tab === x ? 'var(--on-art)' : 'rgba(255,255,255,0.4)',
        borderBottom: tab === x ? '2px solid var(--accent)' : '2px solid transparent'
      }
    }, x === 'queue' ? 'Up next' : 'Lyrics'))), tab === 'queue' ? /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        overflowY: 'auto',
        display: 'flex',
        flexDirection: 'column',
        gap: 2,
        margin: '0 -10px',
        paddingRight: 6
      }
    }, upNext.map((q, i) => {
      const on = q.title === t.title;
      return /*#__PURE__*/React.createElement("button", {
        key: i,
        onClick: () => ctx.playTrack(q),
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          padding: '8px 10px',
          border: 'none',
          background: on ? 'rgba(255,255,255,0.08)' : 'transparent',
          borderRadius: 'var(--radius-md)',
          cursor: 'pointer',
          textAlign: 'left'
        },
        onMouseEnter: e => {
          if (!on) e.currentTarget.style.background = 'rgba(255,255,255,0.05)';
        },
        onMouseLeave: e => {
          if (!on) e.currentTarget.style.background = 'transparent';
        }
      }, /*#__PURE__*/React.createElement(CoverArt, {
        seed: q.seed,
        size: 42,
        radius: "var(--radius-sm)"
      }), /*#__PURE__*/React.createElement("span", {
        style: {
          minWidth: 0,
          flex: 1
        }
      }, /*#__PURE__*/React.createElement("span", {
        style: {
          display: 'block',
          font: 'var(--text-label-font)',
          color: on ? 'var(--accent)' : 'var(--on-art)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap'
        }
      }, q.title), /*#__PURE__*/React.createElement("span", {
        style: {
          display: 'block',
          font: 'var(--text-meta-font)',
          color: 'rgba(255,255,255,0.55)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap'
        }
      }, q.artist)), /*#__PURE__*/React.createElement("span", {
        style: {
          font: 'var(--text-meta-font)',
          fontFamily: 'var(--font-mono)',
          color: 'rgba(255,255,255,0.45)'
        }
      }, q.duration));
    })) : /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        overflowY: 'auto',
        font: 'var(--weight-semibold) 26px/1.5 var(--font-display)',
        color: 'rgba(255,255,255,0.35)',
        letterSpacing: '-0.01em'
      }
    }, /*#__PURE__*/React.createElement("p", {
      style: {
        color: 'var(--on-art)'
      }
    }, "Hold the line, the signal's breaking up"), /*#__PURE__*/React.createElement("p", null, "Through the static I can hear you"), /*#__PURE__*/React.createElement("p", null, "A thousand miles of cobalt overhead"), /*#__PURE__*/React.createElement("p", null, "And the long night carries on"), /*#__PURE__*/React.createElement("p", {
      style: {
        marginTop: 24
      }
    }, "Lyrics provided for demo purposes")))));
  }
  window.NowPlaying = NowPlaying;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/NowPlaying.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/QueuePanel.jsx
try { (() => {
/* QueuePanel — right-docked queue (now playing + up next). */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    CoverArt,
    IconButton,
    Icon
  } = NS;
  function QueueItem({
    q,
    active,
    onPlay
  }) {
    const [h, setH] = React.useState(false);
    return /*#__PURE__*/React.createElement("button", {
      onClick: onPlay,
      onMouseEnter: () => setH(true),
      onMouseLeave: () => setH(false),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 11,
        width: '100%',
        padding: '8px 10px',
        border: 'none',
        background: active ? 'var(--teal-tint)' : h ? 'var(--ink-800)' : 'transparent',
        borderRadius: 'var(--radius-md)',
        cursor: 'pointer',
        textAlign: 'left'
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: q.seed,
      size: 40,
      radius: "var(--radius-sm)"
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        minWidth: 0,
        flex: 1
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        font: 'var(--text-label-font)',
        color: active ? 'var(--accent)' : 'var(--text-primary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, q.title), /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        font: 'var(--text-meta-font)',
        color: 'var(--text-tertiary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, q.artist)), /*#__PURE__*/React.createElement("span", {
      style: {
        opacity: h ? 1 : 0,
        transition: 'var(--t-hover)'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "moreV",
      size: 16,
      style: {
        color: 'var(--text-tertiary)'
      }
    })));
  }
  function QueuePanel({
    ctx
  }) {
    const t = ctx.current;
    const idx = ctx.queue.findIndex(q => q.title === (t && t.title));
    const nowList = t ? [t] : [];
    const next = ctx.queue.slice(idx + 1);
    return /*#__PURE__*/React.createElement("aside", {
      style: {
        width: 'var(--queue-w)',
        flex: 'none',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--bg-chrome)',
        borderLeft: '1px solid var(--line-soft)',
        animation: 'qIn var(--dur-base) var(--ease-out)'
      }
    }, /*#__PURE__*/React.createElement("style", null, `@keyframes qIn{from{transform:translateX(20px);opacity:0}to{transform:translateX(0);opacity:1}}`), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '18px 18px 12px',
        flex: 'none'
      }
    }, /*#__PURE__*/React.createElement("h2", {
      style: {
        font: 'var(--text-h3-font)',
        color: 'var(--text-primary)'
      }
    }, "Queue"), /*#__PURE__*/React.createElement(IconButton, {
      name: "x",
      size: "sm",
      title: "Close queue",
      onClick: ctx.toggleQueue
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        overflowY: 'auto',
        padding: '0 10px 16px'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--weight-semibold) 11px/1 var(--font-ui)',
        letterSpacing: '0.14em',
        textTransform: 'uppercase',
        color: 'var(--text-tertiary)',
        padding: '8px 10px 6px'
      }
    }, "Now playing"), nowList.map((q, i) => /*#__PURE__*/React.createElement(QueueItem, {
      key: 'n' + i,
      q: q,
      active: true,
      onPlay: () => {}
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '16px 10px 6px'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--weight-semibold) 11px/1 var(--font-ui)',
        letterSpacing: '0.14em',
        textTransform: 'uppercase',
        color: 'var(--text-tertiary)'
      }
    }, "Next up"), /*#__PURE__*/React.createElement("button", {
      style: {
        border: 'none',
        background: 'transparent',
        cursor: 'pointer',
        font: 'var(--text-meta-font)',
        color: 'var(--text-tertiary)'
      }
    }, "Clear")), next.map((q, i) => /*#__PURE__*/React.createElement(QueueItem, {
      key: 'x' + i,
      q: q,
      onPlay: () => ctx.playTrack(q)
    }))));
  }
  window.QueuePanel = QueuePanel;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/QueuePanel.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/SearchScreen.jsx
try { (() => {
/* SearchScreen — browse chips when empty, live results when querying. */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    CoverArt,
    AlbumTile,
    TrackRow,
    Chip,
    Icon,
    PlayButton
  } = NS;
  const {
    Page,
    Shelf
  } = window.CelimosCommon;
  const D = window.CELIMOS_DATA;
  const GENRES = ['Ambient', 'Electronic', 'Downtempo', 'Techno', 'Jazz', 'Lo-fi', 'Art Pop', 'IDM', 'Soul', 'House'];
  function BrowseCard({
    label,
    seed
  }) {
    const [h, setH] = React.useState(false);
    return /*#__PURE__*/React.createElement("div", {
      onMouseEnter: () => setH(true),
      onMouseLeave: () => setH(false),
      style: {
        position: 'relative',
        height: 110,
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
        cursor: 'pointer',
        transform: h ? 'translateY(-2px)' : 'none',
        transition: 'var(--t-hover)'
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: seed,
      size: "100%",
      radius: "0",
      style: {
        position: 'absolute',
        inset: 0,
        height: 110
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: 'linear-gradient(120deg, rgba(6,6,7,0.55), rgba(6,6,7,0.05))'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: 16,
        bottom: 14,
        font: 'var(--text-h3-font)',
        color: 'var(--on-art)'
      }
    }, label));
  }
  function SearchScreen({
    ctx
  }) {
    const q = (ctx.query || '').trim().toLowerCase();
    if (!q) {
      return /*#__PURE__*/React.createElement(Page, null, /*#__PURE__*/React.createElement("h1", {
        style: {
          font: 'var(--text-h1-font)',
          letterSpacing: 'var(--tracking-tight)',
          marginBottom: 'var(--space-7)'
        }
      }, "Search"), /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'flex',
          gap: 8,
          flexWrap: 'wrap',
          marginBottom: 'var(--space-9)'
        }
      }, GENRES.slice(0, 6).map(g => /*#__PURE__*/React.createElement(Chip, {
        key: g
      }, g))), /*#__PURE__*/React.createElement("h2", {
        style: {
          font: 'var(--text-h2-font)',
          marginBottom: 'var(--space-5)'
        }
      }, "Browse all"), /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'grid',
          gridTemplateColumns: 'repeat(4, 1fr)',
          gap: 'var(--space-4)'
        }
      }, GENRES.map((g, i) => /*#__PURE__*/React.createElement(BrowseCard, {
        key: g,
        label: g,
        seed: 'genre-' + g
      }))));
    }
    const albums = D.albumList.filter(a => a.title.toLowerCase().includes(q) || a.artist.name.toLowerCase().includes(q));
    const artists = D.artistList.filter(a => a.name.toLowerCase().includes(q) || a.genre.toLowerCase().includes(q));
    const tracks = D.albumList.flatMap(a => a.tracks.map(t => ({
      ...t,
      album: a.title,
      artist: a.artist.name,
      seed: a.seed,
      albumId: a.id
    }))).filter(t => t.title.toLowerCase().includes(q) || t.artist.toLowerCase().includes(q)).slice(0, 5).map((t, i) => ({
      ...t,
      index: i + 1
    }));
    const topArtist = artists[0];
    return /*#__PURE__*/React.createElement(Page, null, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: topArtist ? '0.9fr 1.4fr' : '1fr',
        gap: 'var(--space-7)',
        marginBottom: 'var(--space-9)'
      }
    }, topArtist && /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("h2", {
      style: {
        font: 'var(--text-h2-font)',
        marginBottom: 'var(--space-5)'
      }
    }, "Top result"), /*#__PURE__*/React.createElement("div", {
      onClick: () => ctx.go({
        type: 'artist',
        id: topArtist.name
      }),
      style: {
        position: 'relative',
        background: 'var(--ink-800)',
        border: '1px solid var(--line-soft)',
        borderRadius: 'var(--radius-lg)',
        padding: 22,
        cursor: 'pointer',
        height: 200
      }
    }, /*#__PURE__*/React.createElement(CoverArt, {
      seed: topArtist.seed,
      size: 92,
      radius: "50%",
      shadow: true
    }), /*#__PURE__*/React.createElement("h3", {
      style: {
        font: 'var(--weight-bold) 28px/1 var(--font-display)',
        letterSpacing: '-0.02em',
        margin: '18px 0 8px',
        color: 'var(--text-primary)'
      }
    }, topArtist.name), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-label-font)',
        color: 'var(--text-secondary)'
      }
    }, "Artist \xB7 ", topArtist.genre))), tracks.length > 0 && /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("h2", {
      style: {
        font: 'var(--text-h2-font)',
        marginBottom: 'var(--space-5)'
      }
    }, "Tracks"), tracks.map(t => /*#__PURE__*/React.createElement(TrackRow, {
      key: t.index,
      index: t.index,
      title: t.title,
      artist: t.artist,
      showArt: true,
      seed: t.seed,
      duration: t.duration,
      explicit: t.explicit,
      liked: ctx.isLiked(t),
      active: ctx.current && ctx.current.title === t.title,
      playing: ctx.playing && ctx.current && ctx.current.title === t.title,
      onPlay: () => ctx.playTrack(t),
      onLike: () => ctx.toggleLike(t),
      onMore: () => {}
    })))), albums.length > 0 && /*#__PURE__*/React.createElement(Shelf, {
      title: "Albums"
    }, albums.map(a => /*#__PURE__*/React.createElement(AlbumTile, {
      key: a.id,
      title: a.title,
      subtitle: a.artist.name,
      seed: a.seed,
      size: "var(--tile-lg)",
      onOpen: () => ctx.go({
        type: 'album',
        id: a.id
      }),
      onPlay: () => ctx.playContext(a)
    }))), albums.length === 0 && artists.length === 0 && tracks.length === 0 && /*#__PURE__*/React.createElement("div", {
      style: {
        textAlign: 'center',
        padding: '80px 0',
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "search",
      size: 40,
      style: {
        margin: '0 auto 16px',
        color: 'var(--text-disabled)'
      }
    }), /*#__PURE__*/React.createElement("p", {
      style: {
        font: 'var(--text-h3-font)',
        color: 'var(--text-secondary)'
      }
    }, "No results for \u201C", ctx.query, "\u201D")));
  }
  window.SearchScreen = SearchScreen;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/SearchScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/SettingsScreen.jsx
try { (() => {
/* SettingsScreen — tabbed preferences. The Appearance tab live-reconfigures the
   running app (theme, accent, density, library view, panels, player). */
(function () {
  const NS = window.CelimosDesignSystem_555e0b;
  const {
    Input,
    Switch,
    Button,
    SegmentedControl,
    Select,
    AccentPicker,
    NumberStepper,
    SettingsGroup,
    SettingsRow,
    Badge,
    Avatar,
    Icon
  } = NS;
  const {
    Page
  } = window.CelimosCommon;
  function PreviewSwatch({
    p
  }) {
    // a tiny live preview of the current accent + surfaces
    return /*#__PURE__*/React.createElement("div", {
      "data-theme": p.theme,
      "data-accent": p.accent,
      style: {
        ...(p.accent === 'custom' ? {
          '--accent': p.customAccent
        } : {}),
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: 14,
        borderRadius: 'var(--radius-md)',
        background: 'var(--surface-card)',
        border: '1px solid var(--line)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 38,
        height: 38,
        borderRadius: '50%',
        background: 'var(--accent)',
        boxShadow: '0 4px 14px var(--focus-ring)',
        display: 'grid',
        placeItems: 'center',
        color: 'var(--text-on-accent)'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "play",
      size: 16
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        height: 8,
        width: 120,
        borderRadius: 4,
        background: 'var(--text-primary)',
        opacity: 0.85
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        height: 6,
        width: 80,
        borderRadius: 3,
        background: 'var(--text-secondary)',
        marginTop: 7
      }
    })), /*#__PURE__*/React.createElement("span", {
      style: {
        padding: '3px 9px',
        borderRadius: 'var(--radius-sm)',
        border: '1px solid var(--accent)',
        color: 'var(--accent)',
        font: 'var(--weight-bold) 10px/1 var(--font-ui)',
        letterSpacing: '0.06em'
      }
    }, "LIVE"));
  }
  function AppearanceTab({
    ctx
  }) {
    const p = ctx.prefs,
      set = ctx.setPref;
    return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Theme",
      description: "Celimos is designed for the dark, but the light theme is fully supported."
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Appearance",
      description: "Switch the entire surface palette"
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Dark',
        value: 'dark'
      }, {
        label: 'Light',
        value: 'light'
      }],
      value: p.theme,
      onChange: v => set('theme', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Accent color",
      description: "Drives the play button, highlights, focus rings & progress",
      align: "top"
    }, /*#__PURE__*/React.createElement(AccentPicker, {
      value: p.accent,
      customColor: p.customAccent,
      onChange: (key, color) => {
        set('accent', key);
        if (key === 'custom') set('customAccent', color);
      }
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Preview",
      last: true
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        width: 280
      }
    }, /*#__PURE__*/React.createElement(PreviewSwatch, {
      p: p
    })))), /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Layout",
      description: "Density and panel arrangement."
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Density",
      description: "Compact fits more tracks per screen; spacious gives art room to breathe"
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Compact',
        value: 'compact'
      }, {
        label: 'Balanced',
        value: 'balanced'
      }, {
        label: 'Spacious',
        value: 'spacious'
      }],
      value: p.density,
      onChange: v => set('density', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Text size"
    }, /*#__PURE__*/React.createElement(NumberStepper, {
      value: p.fontScale,
      min: 90,
      max: 115,
      step: 5,
      unit: "%",
      onChange: v => set('fontScale', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Show sidebar",
      description: "Hide for a focused, art-forward window"
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.showSidebar,
      onChange: v => set('showSidebar', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Queue location",
      description: "Dock the queue beside content, or only inside the full-screen player"
    }, /*#__PURE__*/React.createElement(Select, {
      value: p.queueMode,
      onChange: v => set('queueMode', v),
      align: "right",
      options: [{
        label: 'Side panel',
        value: 'panel',
        icon: 'queue'
      }, {
        label: 'In full player',
        value: 'nowplaying',
        icon: 'disc'
      }]
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Play bar",
      last: true
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Docked',
        value: 'docked'
      }, {
        label: 'Floating',
        value: 'floating'
      }],
      value: p.playbarStyle,
      onChange: v => set('playbarStyle', v)
    }))), /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Library",
      description: "How albums and artists are displayed."
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Default view",
      description: "Grid of covers or a compact list"
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Grid',
        value: 'grid'
      }, {
        label: 'List',
        value: 'list'
      }],
      value: p.libraryView,
      onChange: v => set('libraryView', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Cover size",
      description: "Size of album tiles in grid view"
    }, /*#__PURE__*/React.createElement(NumberStepper, {
      value: p.cardSize,
      min: 140,
      max: 260,
      step: 12,
      unit: "px",
      onChange: v => set('cardSize', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Round artist images",
      last: true
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.roundArtistArt,
      onChange: v => set('roundArtistArt', v)
    }))), /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Player",
      description: "The full-screen now-playing experience."
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Background",
      description: "Wash from the cover's color, or a flat surface"
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Cover color',
        value: 'cover'
      }, {
        label: 'Solid',
        value: 'solid'
      }],
      value: p.nowPlayingBg,
      onChange: v => set('nowPlayingBg', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Animated equalizer",
      description: "Show moving bars on the currently playing track",
      last: true
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.showEqualizer,
      onChange: v => set('showEqualizer', v)
    }))));
  }
  function ServerTab({
    ctx
  }) {
    const [host, setHost] = React.useState('https://navidrome.local');
    const [user, setUser] = React.useState('alex');
    return /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Navidrome server",
      description: "Connect to your self-hosted Navidrome / Subsonic library."
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Server URL",
      description: "Your self-hosted endpoint"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        width: 300
      }
    }, /*#__PURE__*/React.createElement(Input, {
      value: host,
      onChange: setHost,
      icon: "server"
    }))), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Username"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        width: 300
      }
    }, /*#__PURE__*/React.createElement(Input, {
      value: user,
      onChange: setUser
    }))), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Password"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        width: 300
      }
    }, /*#__PURE__*/React.createElement(Input, {
      type: "password",
      value: "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022",
      onChange: () => {}
    }))), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Connection",
      description: "Last synced 2 minutes ago \xB7 1,284 albums"
    }, /*#__PURE__*/React.createElement(Badge, {
      tone: "success",
      style: {
        gap: 6
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 7,
        height: 7,
        borderRadius: '50%',
        background: 'var(--success)'
      }
    }), "Connected")), /*#__PURE__*/React.createElement(SettingsRow, {
      last: true
    }, /*#__PURE__*/React.createElement(Button, {
      variant: "secondary"
    }, "Test connection"), /*#__PURE__*/React.createElement(Button, {
      variant: "primary"
    }, "Save & sync")));
  }
  function PlaybackTab({
    ctx
  }) {
    const p = ctx.prefs,
      set = ctx.setPref;
    return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Playback"
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Streaming quality",
      description: "Lossless uses more bandwidth on your connection"
    }, /*#__PURE__*/React.createElement(Select, {
      value: p.quality,
      onChange: v => set('quality', v),
      align: "right",
      options: [{
        label: 'Low · 96 kbps',
        value: 'low'
      }, {
        label: 'High · 320 kbps',
        value: 'high'
      }, {
        label: 'Lossless · FLAC',
        value: 'lossless'
      }]
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Volume normalization",
      description: "Even out loudness across tracks"
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.normalize,
      onChange: v => set('normalize', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Gapless playback"
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.gapless,
      onChange: v => set('gapless', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Crossfade",
      description: "Blend the end of one track into the next"
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.crossfade,
      onChange: v => set('crossfade', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Crossfade length",
      last: true
    }, /*#__PURE__*/React.createElement(NumberStepper, {
      value: p.crossfadeSeconds,
      min: 1,
      max: 12,
      step: 1,
      unit: "s",
      onChange: v => set('crossfadeSeconds', v),
      disabled: !p.crossfade
    }))), /*#__PURE__*/React.createElement(SettingsGroup, {
      title: "Account"
    }, /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Profile"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 12
      }
    }, /*#__PURE__*/React.createElement(Avatar, {
      name: "Alex Rivera",
      size: 36
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--text-label-font)',
        color: 'var(--text-secondary)'
      }
    }, "Alex Rivera"))), /*#__PURE__*/React.createElement(SettingsRow, {
      title: "Scrobble to Last.fm"
    }, /*#__PURE__*/React.createElement(Switch, {
      checked: p.scrobble,
      onChange: v => set('scrobble', v)
    })), /*#__PURE__*/React.createElement(SettingsRow, {
      last: true
    }, /*#__PURE__*/React.createElement(Button, {
      variant: "ghost",
      iconLeft: "settings",
      onClick: ctx.resetPrefs
    }, "Reset to defaults"), /*#__PURE__*/React.createElement(Button, {
      variant: "danger",
      iconLeft: "x"
    }, "Disconnect server"))));
  }
  function SettingsScreen({
    ctx
  }) {
    const [tab, setTab] = React.useState('appearance');
    return /*#__PURE__*/React.createElement(Page, {
      style: {
        maxWidth: 780
      }
    }, /*#__PURE__*/React.createElement("h1", {
      style: {
        font: 'var(--text-h1-font)',
        letterSpacing: 'var(--tracking-tight)',
        marginBottom: 'var(--space-6)'
      }
    }, "Settings"), /*#__PURE__*/React.createElement("div", {
      style: {
        marginBottom: 'var(--space-8)'
      }
    }, /*#__PURE__*/React.createElement(SegmentedControl, {
      options: [{
        label: 'Appearance',
        value: 'appearance'
      }, {
        label: 'Server',
        value: 'server'
      }, {
        label: 'Playback',
        value: 'playback'
      }],
      value: tab,
      onChange: setTab
    })), tab === 'appearance' && /*#__PURE__*/React.createElement(AppearanceTab, {
      ctx: ctx
    }), tab === 'server' && /*#__PURE__*/React.createElement(ServerTab, {
      ctx: ctx
    }), tab === 'playback' && /*#__PURE__*/React.createElement(PlaybackTab, {
      ctx: ctx
    }));
  }
  window.SettingsScreen = SettingsScreen;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/SettingsScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/data.js
try { (() => {
/* Celimos demo catalog — invented artists/albums so the kit ships no
   copyrighted metadata. Covers are generated from the `seed` field. */
(function () {
  const A = (name, seed, genre) => ({
    name,
    seed,
    genre
  });
  const artists = {
    velamont: A('Vela Mont', 'velamont', 'Ambient / Electronic'),
    hollow: A('Hollow Coast', 'hollowcoast', 'Downtempo'),
    reign: A('Nadia Reign', 'nadiareign', 'Art Pop'),
    lowtide: A('Lowtide Collective', 'lowtide', 'Electronic'),
    kasimir: A('Kasimir', 'kasimir', 'Techno'),
    pinegrove: A('Pine Grove Tapes', 'pinegrove', 'Lo-fi'),
    aerial: A('Aerial Method', 'aerialmethod', 'IDM'),
    saudade: A('Saudade', 'saudade', 'Jazz / Soul')
  };
  const T = (title, dur, opts = {}) => ({
    title,
    duration: dur,
    explicit: !!opts.explicit,
    liked: !!opts.liked,
    plays: opts.plays
  });
  const albums = {
    longnight: {
      id: 'longnight',
      title: 'The Long Night',
      artist: artists.velamont,
      year: 2024,
      seed: 'velamont-longnight',
      type: 'Album',
      tracks: [T('Aurora Line', '4:12', {
        liked: true
      }), T('Glass Harbour', '5:38'), T('Meridian', '3:54', {
        liked: true
      }), T('Salt & Static', '6:07'), T('Northern Drift', '4:45'), T('The Long Night', '7:21'), T('Afterglow', '5:02'), T('Low Sun', '3:33')]
    },
    tidalcode: {
      id: 'tidalcode',
      title: 'Tidal Code',
      artist: artists.hollow,
      year: 2023,
      seed: 'hollow-tidalcode',
      type: 'Album',
      tracks: [T('Undertow', '3:48', {
        liked: true
      }), T('Cobalt', '4:22'), T('Driftwood', '5:10', {
        explicit: true
      }), T('Harbour Lights', '4:01'), T('Sea Change', '6:33', {
        liked: true
      }), T('Slow Tide', '3:57')]
    },
    papermoons: {
      id: 'papermoons',
      title: 'Paper Moons',
      artist: artists.reign,
      year: 2025,
      seed: 'reign-papermoons',
      type: 'Album',
      tracks: [T('Velvet Hour', '3:21', {
        explicit: true,
        liked: true
      }), T('Paper Moons', '4:08'), T('Carousel', '3:44'), T('Neon Rain', '4:55', {
        explicit: true
      }), T('Goldenrod', '3:12'), T('Closing Time', '5:18', {
        liked: true
      })]
    },
    signals: {
      id: 'signals',
      title: 'Signals',
      artist: artists.lowtide,
      year: 2022,
      seed: 'lowtide-signals',
      type: 'Album',
      tracks: [T('Beacon', '5:44'), T('Phosphor', '4:30', {
        liked: true
      }), T('Transit', '6:12'), T('Halo', '4:08'), T('Signals', '7:02')]
    },
    monolith: {
      id: 'monolith',
      title: 'Monolith',
      artist: artists.kasimir,
      year: 2024,
      seed: 'kasimir-monolith',
      type: 'Album',
      tracks: [T('Granite', '6:48', {
        liked: true
      }), T('Concrete', '5:21'), T('Rebar', '4:59', {
        explicit: true
      }), T('Monolith', '8:14'), T('Aftershock', '6:02')]
    },
    warmstatic: {
      id: 'warmstatic',
      title: 'Warm Static',
      artist: artists.pinegrove,
      year: 2023,
      seed: 'pine-warmstatic',
      type: 'Album',
      tracks: [T('Cassette Sun', '2:58', {
        liked: true
      }), T('Dust', '3:14'), T('Warm Static', '3:41'), T('Backyard', '2:47'), T('Hazy', '3:09', {
        liked: true
      })]
    }
  };
  const albumList = Object.values(albums);
  const playlists = [{
    id: 'focus',
    title: 'Deep Focus',
    desc: 'Ambient textures for long sessions',
    seed: 'pl-focus',
    count: 48,
    by: 'Celimos',
    tracksFrom: ['longnight', 'signals', 'warmstatic']
  }, {
    id: 'latenight',
    title: 'Late Night Drive',
    desc: 'Downtempo for empty roads',
    seed: 'pl-latenight',
    count: 36,
    by: 'Celimos',
    tracksFrom: ['tidalcode', 'papermoons']
  }, {
    id: 'mintapes',
    title: 'Midnight Tapes',
    desc: 'Your lo-fi rotation',
    seed: 'pl-mintapes',
    count: 62,
    by: 'You',
    tracksFrom: ['warmstatic', 'longnight']
  }, {
    id: 'pulse',
    title: 'Pulse',
    desc: 'Techno that moves',
    seed: 'pl-pulse',
    count: 54,
    by: 'Celimos',
    tracksFrom: ['monolith', 'signals']
  }];

  // Flatten a "now playing" queue
  function buildQueue() {
    const q = [];
    ['longnight', 'tidalcode', 'papermoons', 'monolith'].forEach(aid => {
      const al = albums[aid];
      al.tracks.slice(0, 3).forEach(t => q.push({
        ...t,
        album: al.title,
        artist: al.artist.name,
        seed: al.seed
      }));
    });
    return q;
  }
  window.CELIMOS_DATA = {
    artists,
    artistList: Object.values(artists),
    albums,
    albumList,
    playlists,
    buildQueue
  };
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/data.js", error: String((e && e.message) || e) }); }

// ui_kits/celimos-app/preferences.js
try { (() => {
/* Celimos preferences — the "configure everything" layer (Feishin-style).
   A tiny store persisted to localStorage that the app reads to reconfigure
   theme, accent, density, library view, panels, and the player. */
(function () {
  const KEY = 'celimos.prefs.v1';
  const defaults = {
    // appearance
    theme: 'dark',
    // 'dark' | 'light'
    accent: 'teal',
    // teal | indigo | emerald | amber | gold | crimson | custom
    customAccent: '#8b5cff',
    density: 'spacious',
    // 'spacious' | 'balanced' | 'compact'
    fontScale: 100,
    // 90–115 (%)

    // library
    libraryView: 'grid',
    // 'grid' | 'list'
    cardSize: 200,
    // px tile size in grid view
    roundArtistArt: true,
    // layout / panels
    showSidebar: true,
    queueMode: 'panel',
    // 'panel' (right dock) | 'nowplaying' (in full player)
    playbarStyle: 'docked',
    // 'docked' | 'floating'

    // player
    nowPlayingBg: 'cover',
    // 'cover' (hue wash) | 'solid'
    showEqualizer: true,
    // playback (mirrors settings)
    quality: 'lossless',
    normalize: true,
    gapless: true,
    crossfade: false,
    crossfadeSeconds: 6,
    scrobble: true
  };

  // Density → CSS custom-property overrides applied on the app root.
  const DENSITY = {
    spacious: {
      '--content-pad': '40px',
      '--row-h': '56px',
      '--shelf-gap': '24px',
      '--tile-lg': '200px'
    },
    balanced: {
      '--content-pad': '32px',
      '--row-h': '50px',
      '--shelf-gap': '18px',
      '--tile-lg': '176px'
    },
    compact: {
      '--content-pad': '24px',
      '--row-h': '44px',
      '--shelf-gap': '14px',
      '--tile-lg': '152px'
    }
  };
  function load() {
    try {
      return {
        ...defaults,
        ...(JSON.parse(localStorage.getItem(KEY)) || {})
      };
    } catch (e) {
      return {
        ...defaults
      };
    }
  }
  function save(p) {
    try {
      localStorage.setItem(KEY, JSON.stringify(p));
    } catch (e) {}
  }

  /* Compute the style object (data-* + CSS vars) to spread on the app root. */
  function rootProps(p) {
    const vars = {
      ...DENSITY[p.density]
    };
    vars['--app-font-scale'] = (p.fontScale / 100).toString();
    if (p.accent === 'custom') vars['--accent'] = p.customAccent;
    // grid tile size override (library grid uses --tile-lg already; expose cardSize too)
    vars['--tile-lg'] = (p.cardSize || 200) + 'px';
    return {
      'data-theme': p.theme,
      'data-accent': p.accent,
      style: vars
    };
  }
  window.CELIMOS_PREFS = {
    defaults,
    DENSITY,
    load,
    save,
    rootProps
  };
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/celimos-app/preferences.js", error: String((e && e.message) || e) }); }

__ds_ns.ACCENT_PRESETS = __ds_scope.ACCENT_PRESETS;

__ds_ns.AccentPicker = __ds_scope.AccentPicker;

__ds_ns.Avatar = __ds_scope.Avatar;

__ds_ns.Badge = __ds_scope.Badge;

__ds_ns.ExplicitBadge = __ds_scope.ExplicitBadge;

__ds_ns.QualityBadge = __ds_scope.QualityBadge;

__ds_ns.Button = __ds_scope.Button;

__ds_ns.Chip = __ds_scope.Chip;

__ds_ns.Icon = __ds_scope.Icon;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.Input = __ds_scope.Input;

__ds_ns.NumberStepper = __ds_scope.NumberStepper;

__ds_ns.PlayButton = __ds_scope.PlayButton;

__ds_ns.SegmentedControl = __ds_scope.SegmentedControl;

__ds_ns.Select = __ds_scope.Select;

__ds_ns.SettingsGroup = __ds_scope.SettingsGroup;

__ds_ns.SettingsRow = __ds_scope.SettingsRow;

__ds_ns.Slider = __ds_scope.Slider;

__ds_ns.Switch = __ds_scope.Switch;

__ds_ns.Tabs = __ds_scope.Tabs;

__ds_ns.AlbumTile = __ds_scope.AlbumTile;

__ds_ns.CoverArt = __ds_scope.CoverArt;

__ds_ns.TrackRow = __ds_scope.TrackRow;

})();
