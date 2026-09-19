import { FileCode2 } from "lucide-react";

interface Props {
  /** Relative paths of the mod's ini files. */
  fileNames: string[];
  /** Currently selected file, or null for "All files". */
  selected: string | null;
  onSelect: (value: string | null) => void;
}

/**
 * Row of chips for picking which ini file's contents to show. "All files" is
 * the default and covers the approachable case; per-file chips serve users who
 * navigate by ini. Only rendered when a mod has more than one ini file.
 */
export default function IniFileSelector({ fileNames, selected, onSelect }: Props) {
  if (fileNames.length <= 1) return null;

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Chip active={selected === null} onClick={() => onSelect(null)} label="All files" />
      {fileNames.map((name) => (
        <Chip
          key={name}
          active={selected === name}
          onClick={() => onSelect(name)}
          label={name}
          icon={<FileCode2 size={11} />}
        />
      ))}
    </div>
  );
}

function Chip({
  active,
  onClick,
  label,
  icon,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
  icon?: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`game-control flex items-center gap-1 px-2 py-0.5 text-[11px] font-medium border transition-colors ${
        active
          ? "bg-game2/10 border-game2/40 text-game2"
          : "bg-surface-1 border-surface-3 text-text-secondary hover:text-text-primary"
      }`}
      title={label}
    >
      {icon}
      <span className="max-w-[9rem] truncate font-mono">{label}</span>
    </button>
  );
}
