type FridayAuraProps = {
  state?: "idle" | "listening" | "thinking" | "speaking";
};

export function FridayAura({ state = "idle" }: FridayAuraProps) {
  const stateClass = state !== "idle" ? ` is-${state}` : "";
  return (
    <div className={`friday-aura${stateClass}`} aria-hidden>
      <div className="friday-aura-shell" />
      <div className="friday-aura-ring" />
      <div className="friday-aura-core" />
      <div className="friday-aura-sweep" />
    </div>
  );
}
