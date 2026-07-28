import tickMascot from "../assets/tick-mascot.png";

export function TickMascot() {
  return (
    <div className="tick-mascot" aria-hidden="true">
      <img src={tickMascot} alt="" draggable={false} />
    </div>
  );
}
