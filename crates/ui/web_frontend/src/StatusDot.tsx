export function StatusDot({ status = "online" }: { status?: "online" | "offline" }) {
    const colorClass = status === "online" ? "bg-green-500" : "bg-gray-400";
    const pingClass = status === "online" ? "bg-green-400" : "hidden";
    return (
        <span className="relative flex h-3 w-3">
            <span className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${pingClass}`}></span>
            <span className={`relative inline-flex rounded-full h-3 w-3 ${colorClass}`}></span>
        </span>
    )
}