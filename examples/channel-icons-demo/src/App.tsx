import { useState } from "react";
import {
  ChannelIcon,
  ChannelIconPicker,
  enLabels,
  koLabels,
  type ChannelIconValue,
} from "channel-icons";

interface Project {
  id: string;
  name: string;
  color: string;
}

interface Channel extends ChannelIconValue {
  id: string;
  name: string;
  projectId: string;
}

const PROJECTS: Project[] = [
  { id: "apollo", name: "Apollo", color: "#2563eb" },
  { id: "borealis", name: "Borealis", color: "#0d9488" },
];

const INITIAL_CHANNELS: Channel[] = [
  { id: "1", name: "announcements", projectId: "apollo", icon: "notice", color: null },
  { id: "2", name: "engineering", projectId: "apollo", icon: "dev", color: null },
  // A color outside the default palette stays selectable as "Current color".
  { id: "3", name: "launch-party", projectId: "borealis", icon: "celebrate", color: "#db2777" },
  { id: "4", name: "support", projectId: "borealis", icon: "support", color: null },
];

const FALLBACK_COLOR = "#475569";

/** `color: null` means "Auto": the tile takes the color of the channel's project. */
function projectColor(channel: Channel): string {
  return PROJECTS.find((project) => project.id === channel.projectId)?.color ?? FALLBACK_COLOR;
}

function tileColor(channel: Channel): string {
  return channel.color ?? projectColor(channel);
}

export function App() {
  const [channels, setChannels] = useState(INITIAL_CHANNELS);
  const [selectedId, setSelectedId] = useState(INITIAL_CHANNELS[0]?.id);
  const [korean, setKorean] = useState(false);

  const selected = channels.find((channel) => channel.id === selectedId) ?? channels[0];

  function update(id: string, next: ChannelIconValue) {
    setChannels((list) =>
      list.map((channel) => (channel.id === id ? { ...channel, ...next } : channel)),
    );
  }

  return (
    <main className="demo">
      <h1>channel-icons demo</h1>
      <p className="lead">
        Pick a channel, then change its icon and background. &ldquo;Auto&rdquo; follows the color of
        the channel&rsquo;s project.
      </p>

      <label className="toggle">
        <input
          type="checkbox"
          checked={korean}
          onChange={(event) => setKorean(event.target.checked)}
        />
        Korean labels
      </label>

      <div className="layout">
        <section aria-labelledby="channels-heading">
          <h2 id="channels-heading">Channels</h2>
          <ul className="channel-list">
            {channels.map((channel) => (
              <li key={channel.id}>
                <button
                  type="button"
                  className="channel-row"
                  aria-current={channel.id === selected?.id ? "true" : undefined}
                  onClick={() => setSelectedId(channel.id)}
                >
                  <ChannelIcon icon={channel.icon} color={tileColor(channel)} size={28} />
                  <span className="channel-name">{channel.name}</span>
                  <span className="channel-project">
                    {PROJECTS.find((project) => project.id === channel.projectId)?.name}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </section>

        {selected ? (
          <section aria-labelledby="appearance-heading">
            <h2 id="appearance-heading">Appearance of #{selected.name}</h2>
            <ChannelIconPicker
              value={{ icon: selected.icon, color: selected.color }}
              onChange={(next) => update(selected.id, next)}
              autoColor={projectColor(selected)}
              labels={korean ? koLabels : enLabels}
            />
            <p className="stored">
              Stored value: <code>{JSON.stringify({ icon: selected.icon, color: selected.color })}</code>
            </p>
          </section>
        ) : null}
      </div>
    </main>
  );
}
