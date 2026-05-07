import mikuMoe from "../assets/miku-moe.png";

export function MikuMascot() {
  return (
    <div className="miku-mascot" aria-hidden="true">
      <img src={mikuMoe} alt="" draggable={false} />
    </div>
  );
}
