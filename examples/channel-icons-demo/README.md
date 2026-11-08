# channel-icons demo

A small Vite + React page that uses the [`channel-icons`](../../packages/channel-icons) tile and picker the way an app would:

- a list of channels, each drawn with `<ChannelIcon />`;
- `<ChannelIconPicker />` editing the selected channel;
- **Auto** background: a channel stores `color: null` and the page resolves it to the color of the channel's project, so changing a project color recolors every Auto channel;
- a Korean/English label switch using `koLabels` and `enLabels`.

The value the picker produces is plain data (`{ icon: "dev", color: null }`), shown under the picker, so you can store it anywhere.

## Run it

From the repository root:

```sh
pnpm install
pnpm --filter channel-icons build    # the demo uses the built package
pnpm --filter channel-icons-demo dev
```

Open the address Vite prints. For a production build, run `pnpm --filter channel-icons-demo build`; the output is in `examples/channel-icons-demo/dist` and uses relative paths, so it can be served from any folder.
