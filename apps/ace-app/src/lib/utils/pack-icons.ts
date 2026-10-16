import {
    EyeOff,
    Play,
    Video,
    Music,
    Mic,
    Image,
    Film,
    Tv,
    Podcast,
    MessageSquare,
    MessageCircle,
    Heart,
    ThumbsUp,
    Users,
    CircleUser,
    AtSign,
    Gamepad2,
    LayoutGrid,
    Dice1,
    Trophy,
    Sword,
    Puzzle,
    CircleDollarSign,
    DollarSign,
    TrendingUp,
    Percent,
    ShoppingCart,
    ShoppingBag,
    Tag,
    Receipt,
    Newspaper,
    Book,
    BookOpen,
    FileText,
    Rss,
    Brain,
    Clock,
    Flame,
    Coffee,
    GraduationCap,
    Pencil,
    ChartNoAxesColumnIncreasing,
    Target,
    Globe,
    Shield,
    Ban,
    Star,
    Folder,
    List,
    Code,
    Terminal,
    Wifi,
    Laptop,
    Smartphone,
    Lock,
    TriangleAlert,
    Spade,
    Download,
    type LucideIcon,
} from "@lucide/svelte";

/**
 * Maps slug strings (the canonical engine values) to Lucide icon components.
 * This is the single source of truth for icon rendering in ace-app.
 */
export const LIST_ICON_MAP: Record<string, LucideIcon> = {
    // Content & Media
    adult: EyeOff,
    streaming: Play,
    video: Video,
    music: Music,
    microphone: Mic,
    photo: Image,
    movie: Film,
    tv: Tv,
    podcast: Podcast,

    // Social & Communication
    social: MessageSquare,
    messaging: MessageCircle,
    message: MessageCircle,
    heart: Heart,
    thumbup: ThumbsUp,
    users: Users,
    profile: CircleUser,
    mention: AtSign,

    // Gaming & Entertainment
    gaming: Gamepad2,
    cards: Spade,
    dice: Dice1,
    trophy: Trophy,
    sword: Sword,
    puzzle: Puzzle,

    // Gambling & Risk
    gambling: CircleDollarSign,
    coin: DollarSign,
    trading: TrendingUp,
    percentage: Percent,

    // Shopping & Commerce
    "shopping-cart": ShoppingCart,
    "shopping-bag": ShoppingBag,
    shopping: ShoppingCart,
    tag: Tag,
    receipt: Receipt,

    // Security & Risk
    phishing: TriangleAlert,
    piracy: Download,

    // News & Information
    news: Newspaper,
    book: Book,
    "book-open": BookOpen,
    article: FileText,
    rss: Rss,

    // Productivity & Focus
    brain: Brain,
    clock: Clock,
    flame: Flame,
    coffee: Coffee,
    school: GraduationCap,
    pencil: Pencil,
    chart: ChartNoAxesColumnIncreasing,
    target: Target,

    // Tech & General
    globe: Globe,
    shield: Shield,
    ban: Ban,
    star: Star,
    folder: Folder,
    list: List,
    code: Code,
    terminal: Terminal,
    wifi: Wifi,
    laptop: Laptop,
    mobile: Smartphone,

    // Adult & Sensitive
    lock: Lock,
    warning: TriangleAlert,

    // Legacy Tabler-icon keys (backward compatibility)
    "ti-eye-off": EyeOff,
    "ti-messages": MessageSquare,
    "ti-device-gamepad-2": Gamepad2,
    "ti-cards": Spade,
    "ti-player-play": Play,
    "ti-news": Newspaper,
};

export const FALLBACK_ICON: LucideIcon = Shield;

/**
 * Groups of icon slugs by category, for use in picker UIs.
 */
export const LIST_ICONS_GROUPED: Array<{
    category: string;
    slugs: string[];
}> = [
    {
        category: "Content & Media",
        slugs: [
            "adult",
            "streaming",
            "video",
            "music",
            "microphone",
            "photo",
            "movie",
            "tv",
            "podcast",
        ],
    },
    {
        category: "Social & Communication",
        slugs: [
            "social",
            "message",
            "heart",
            "thumbup",
            "users",
            "profile",
            "mention",
        ],
    },
    {
        category: "Gaming & Entertainment",
        slugs: ["gaming", "cards", "dice", "trophy", "sword", "puzzle"],
    },
    {
        category: "Gambling & Risk",
        slugs: ["gambling", "coin", "trading", "percentage"],
    },
    {
        category: "Shopping & Commerce",
        slugs: ["shopping-cart", "shopping-bag", "tag", "receipt"],
    },
    {
        category: "News & Information",
        slugs: ["news", "book", "book-open", "article", "rss"],
    },
    {
        category: "Productivity & Focus",
        slugs: [
            "brain",
            "clock",
            "flame",
            "coffee",
            "school",
            "pencil",
            "chart",
            "target",
        ],
    },
    {
        category: "Tech & General",
        slugs: [
            "globe",
            "shield",
            "ban",
            "star",
            "folder",
            "list",
            "code",
            "terminal",
            "wifi",
            "laptop",
            "mobile",
        ],
    },
    {
        category: "Adult & Sensitive",
        slugs: ["adult", "lock", "warning"],
    },
];

/**
 * Resolve a slug to a Lucide icon component.
 * Returns `FALLBACK_ICON` if the slug is null, undefined, or unrecognized.
 */
export function getListIcon(slug: string | null | undefined): LucideIcon {
    if (!slug) return FALLBACK_ICON;
    return LIST_ICON_MAP[slug] ?? FALLBACK_ICON;
}

/**
 * Alias for getListIcon, using the "pack" naming convention.
 */
export const getPackIcon = getListIcon;
