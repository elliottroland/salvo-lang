package salvo.codegen;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

import software.amazon.smithy.aws.traits.ServiceTrait;
import software.amazon.smithy.kotlin.codegen.core.NamingKt;
import software.amazon.smithy.kotlin.codegen.utils.CaseUtilsKt;
import software.amazon.smithy.model.Model;
import software.amazon.smithy.model.knowledge.TopDownIndex;
import software.amazon.smithy.model.shapes.BlobShape;
import software.amazon.smithy.model.shapes.BooleanShape;
import software.amazon.smithy.model.shapes.ByteShape;
import software.amazon.smithy.model.shapes.DoubleShape;
import software.amazon.smithy.model.shapes.EnumShape;
import software.amazon.smithy.model.shapes.FloatShape;
import software.amazon.smithy.model.shapes.IntegerShape;
import software.amazon.smithy.model.shapes.ListShape;
import software.amazon.smithy.model.shapes.LongShape;
import software.amazon.smithy.model.shapes.MapShape;
import software.amazon.smithy.model.shapes.MemberShape;
import software.amazon.smithy.model.shapes.OperationShape;
import software.amazon.smithy.model.shapes.ServiceShape;
import software.amazon.smithy.model.shapes.Shape;
import software.amazon.smithy.model.shapes.ShapeId;
import software.amazon.smithy.model.shapes.ShortShape;
import software.amazon.smithy.model.shapes.StringShape;
import software.amazon.smithy.model.shapes.StructureShape;
import software.amazon.smithy.model.shapes.TimestampShape;
import software.amazon.smithy.model.traits.DefaultTrait;
import software.amazon.smithy.model.traits.DocumentationTrait;
import software.amazon.smithy.model.traits.HttpHeaderTrait;
import software.amazon.smithy.model.traits.RequiredTrait;
import software.amazon.smithy.model.traits.StreamingTrait;
import software.amazon.smithy.model.traits.TitleTrait;

/**
 * Walks one service's selected operations and renders four files:
 *
 * <ul>
 *   <li>{@code <module>.sv} — the Salvo surface: enums (unions of unit tags
 *       namespaced under the type, plus an open {@code Unknown} arm), structures,
 *       the service's error union, the effect, and a recording fake;</li>
 *   <li>{@code <module>/host.sv} — the {@code threadsafe platform handler} over
 *       the platform's SDK;</li>
 *   <li>{@code platform/<module>/host.rs} and {@code .kt} — the host glue, which
 *       converts between the Salvo values and the SDK's, runs each call on the
 *       SDK's own async machinery and completes the {@code Reply} from there.</li>
 * </ul>
 *
 * <p>A {@code @streaming} blob is a {@code stream.InStream}, which makes the
 * structure holding it a {@code linear struct} with a {@code close} beside it;
 * the glue registers a response body in the host's stream table and reads a
 * request body out of it [stream-table]. A timestamp is a {@code time.Instant}.
 *
 * <p>Anything the model uses that this generator does not map yet — documents,
 * event streams, unions, big numbers — stops generation with an error naming
 * the shape, rather than emitting something approximate.
 */
final class Generator {
    private final Model model;
    private final Settings settings;
    private final ServiceShape service;
    private final List<OperationShape> operations = new ArrayList<>();
    /** Structures in first-reached order, with the Salvo name each renders as. */
    private final Map<ShapeId, String> structs = new LinkedHashMap<>();
    /** Enums (as value types) in first-reached order. */
    private final Map<ShapeId, String> enums = new LinkedHashMap<>();
    /** Every modeled error of the selected operations, by Salvo name. */
    private final TreeSet<String> errorNames = new TreeSet<>();
    private final Map<String, StructureShape> errorShapes = new TreeMap<>();
    private final Set<ShapeId> inputs = new LinkedHashSet<>();
    private final Set<ShapeId> outputs = new LinkedHashSet<>();
    /** The `@streaming` member of each structure that has one (Smithy allows one, top-level). */
    private final Map<ShapeId, MemberShape> streaming = new LinkedHashMap<>();
    /** Whether any selected shape holds a timestamp. */
    private boolean usesTime = false;

    Generator(Model model, Settings settings) {
        this.model = model;
        this.settings = settings;
        this.service = model.expectShape(settings.service(), ServiceShape.class);
    }

    // ================================================================ walk ===

    Map<String, String> generate() {
        Set<OperationShape> all = TopDownIndex.of(model).getContainedOperations(service);
        for (String name : settings.operations()) {
            OperationShape op = all.stream()
                    .filter(o -> o.getId().getName().equals(name))
                    .findFirst()
                    .orElseThrow(() -> fail(name, "is not an operation of " + service.getId()));
            operations.add(op);
        }
        for (OperationShape op : operations) {
            input(op).ifPresent(s -> {
                inputs.add(s.getId());
                structs.put(s.getId(), op.getId().getName() + "Input");
                walkStruct(s);
            });
            output(op).ifPresent(s -> {
                outputs.add(s.getId());
                structs.put(s.getId(), op.getId().getName() + "Output");
                walkStruct(s);
            });
            for (ShapeId e : op.getErrors(service)) {
                StructureShape es = model.expectShape(e, StructureShape.class);
                errorNames.add(e.getName());
                errorShapes.put(e.getName(), es);
                walkStruct(es);
            }
        }
        Map<String, String> files = new LinkedHashMap<>();
        String path = settings.module().replace('.', '/');
        files.put(path + ".sv", salvoSurface());
        files.put(path + "/host.sv", salvoHost());
        files.put("platform/" + path + "/host.rs", rustGlue());
        files.put("platform/" + path + "/host.kt", kotlinGlue());
        return files;
    }

    private Optional<StructureShape> input(OperationShape op) {
        return op.getInput().filter(id -> !id.toString().equals("smithy.api#Unit"))
                .map(id -> model.expectShape(id, StructureShape.class));
    }

    private Optional<StructureShape> output(OperationShape op) {
        return op.getOutput().filter(id -> !id.toString().equals("smithy.api#Unit"))
                .map(id -> model.expectShape(id, StructureShape.class));
    }

    /** A structure's members in model order, without the ones the settings omit. */
    private List<MemberShape> members(StructureShape s) {
        List<MemberShape> out = new ArrayList<>();
        for (MemberShape m : s.getAllMembers().values()) {
            if (!settings.omitMembers().contains(m.getId().toString())) {
                out.add(m);
            } else if (m.hasTrait(RequiredTrait.class)) {
                throw fail(m.getId().toString(), "is required, so it cannot be omitted");
            }
        }
        return out;
    }

    private void walkStruct(StructureShape s) {
        for (MemberShape m : members(s)) {
            walkType(model.expectShape(m.getTarget()), m);
        }
    }

    private void walkType(Shape t, MemberShape via) {
        if (t.hasTrait(StreamingTrait.class)) {
            // Smithy puts a streaming member only at the top of an operation's
            // input or output (and one per structure); an event stream is a
            // streaming *union*, which is not mapped yet.
            ShapeId holder = via.getContainer();
            if (!(t instanceof BlobShape)) {
                throw fail(via.getId().toString(), "is an event stream, which this generator does not map yet");
            }
            if (!inputs.contains(holder) && !outputs.contains(holder)) {
                throw fail(via.getId().toString(), "is a streaming member outside an operation's input or output");
            }
            if (streaming.containsKey(holder) && !streaming.get(holder).equals(via)) {
                throw fail(holder.toString(), "has two streaming members");
            }
            streaming.put(holder, via);
            return;
        }
        if (t instanceof TimestampShape) {
            usesTime = true;
            return;
        }
        if (t instanceof EnumShape e) {
            enums.putIfAbsent(e.getId(), e.getId().getName());
        } else if (t instanceof StructureShape s) {
            if (!structs.containsKey(s.getId()) && !s.hasTrait("smithy.api#error")) {
                structs.put(s.getId(), s.getId().getName());
                walkStruct(s);
            }
        } else if (t instanceof ListShape l) {
            walkType(model.expectShape(l.getMember().getTarget()), l.getMember());
        } else if (t instanceof MapShape mp) {
            Shape key = model.expectShape(mp.getKey().getTarget());
            if (!(key instanceof StringShape)) {
                throw fail(mp.getId().toString(), "has a key that is not a string");
            }
            walkType(model.expectShape(mp.getValue().getTarget()), mp.getValue());
        } else if (!(t instanceof StringShape || t instanceof IntegerShape || t instanceof ShortShape
                || t instanceof ByteShape || t instanceof LongShape || t instanceof BooleanShape
                || t instanceof FloatShape || t instanceof DoubleShape || t instanceof BlobShape)) {
            throw fail(via.getId().toString(), "targets a " + t.getType() + ", which this generator does not map yet");
        }
    }

    private static IllegalStateException fail(String what, String why) {
        return new IllegalStateException("salvo-client-codegen: `" + what + "` " + why);
    }

    // =============================================================== names ===

    private static final Set<String> SALVO_KEYWORDS = Set.of(
            "fn", "let", "struct", "qualifier", "effect", "handler", "params", "type", "intrinsic",
            "platform", "import", "as", "of", "with", "canbe", "provenance", "refn", "rename", "is",
            "if", "elif", "else", "when", "while", "for", "in", "return", "break", "continue", "state",
            "use", "try", "true", "false", "proj", "once", "linear", "holds", "export", "copy");
    private static final Set<String> RUST_KEYWORDS = Set.of(
            "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do",
            "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let",
            "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
            "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
            "virtual", "where", "while", "yield");
    private static final Set<String> KOTLIN_KEYWORDS = Set.of(
            "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in",
            "interface", "is", "null", "object", "package", "return", "super", "this", "throw", "true",
            "try", "typealias", "typeof", "val", "var", "when", "while");

    /** smithy-rs's snake case: the shared word-boundary splitter, lowered and joined. */
    static String snake(String name) {
        return String.join("_", CaseUtilsKt.splitOnWordBoundaries(name)).toLowerCase();
    }

    /**
     * [platform-factory] A factory name as the compiler derives it
     * (`salvo_core::abi::snake`): `_` before every uppercase letter but the
     * first, lowercased — `SqsError` → `sqs_error`.
     */
    static String factorySnake(String name) {
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < name.length(); i++) {
            char c = name.charAt(i);
            if (Character.isUpperCase(c)) {
                if (i > 0 && out.charAt(out.length() - 1) != '_') out.append('_');
                out.append(Character.toLowerCase(c));
            } else {
                out.append(c);
            }
        }
        return out.toString();
    }

    /** [platform-factory] The factory object of a member (`salvo_core::abi::upper_camel`). */
    static String upperCamel(String snake) {
        StringBuilder out = new StringBuilder();
        for (String part : snake.split("_")) {
            if (part.isEmpty()) continue;
            out.append(Character.toUpperCase(part.charAt(0))).append(part.substring(1));
        }
        return out.toString();
    }

    /** [platform-factory] An operation's factory object: `GetQueueUrl`. */
    private String opObject(OperationShape op) {
        return upperCamel(effectOp(op));
    }

    /** [platform-factory] The Kotlin factory of the failure union's arm for `struct`. */
    private String ktFailure(String struct) {
        return errorUnion() + "s." + ktIdent(factorySnake(struct));
    }

    /** [platform-factory] The Rust factory of the failure union's arm for `struct`. */
    private String rsFailure(String struct) {
        return errorUnion() + "::" + rsIdent(factorySnake(struct));
    }

    private static String pascal(String name) {
        return CaseUtilsKt.toPascalCase(name);
    }

    /** A member's Salvo field name: snake case, a Salvo keyword suffixed with `_`. */
    private static String salvoField(MemberShape m) {
        String s = snake(m.getMemberName());
        return SALVO_KEYWORDS.contains(s) ? s + "_" : s;
    }

    /** A Salvo name as the Rust emitter spells it (`rs_ident`). */
    private static String rsIdent(String salvo) {
        return RUST_KEYWORDS.contains(salvo) ? "r#" + salvo : salvo;
    }

    /** A Salvo name as the Kotlin emitter spells it (`kt_ident`). */
    /**
     * A Salvo value name as the Kotlin backend writes it: camel case
     * ([kt-camel], `salvo_core::case::camel` — an `_` before a lowercase
     * letter dropped and the letter uppercased), back-quoted when a keyword.
     */
    private static String ktIdent(String salvo) {
        String camel = ktCamel(salvo);
        return KOTLIN_KEYWORDS.contains(camel) ? "`" + camel + "`" : camel;
    }

    private static String ktCamel(String salvo) {
        if (salvo.isEmpty() || !Character.isLowerCase(salvo.charAt(0))) return salvo;
        int cut = salvo.indexOf("__");
        String head = cut < 0 ? salvo : salvo.substring(0, cut);
        String tail = cut < 0 ? "" : salvo.substring(cut);
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < head.length(); i++) {
            char c = head.charAt(i);
            if (c == '_' && i + 1 < head.length() && Character.isLowerCase(head.charAt(i + 1))) {
                out.append(Character.toUpperCase(head.charAt(++i)));
            } else {
                out.append(c);
            }
        }
        return out.append(tail).toString();
    }

    /** The SDK's Rust accessor for a member. */
    private static String rsAccessor(MemberShape m) {
        String s = snake(m.getMemberName());
        return RUST_KEYWORDS.contains(s) ? "r#" + s : s;
    }

    /** The SDK's Kotlin property for a member — smithy-kotlin's own rule. */
    private static String ktMember(MemberShape m) {
        return NamingKt.defaultName(m);
    }

    private String rsCrate() {
        return settings.rustCrate();
    }

    private String ktModel() {
        return settings.kotlinPackage() + ".model";
    }

    private String ktClient() {
        String sdkId = service.getTrait(ServiceTrait.class).map(ServiceTrait::getSdkId)
                .orElse(service.getId().getName());
        return settings.kotlinPackage() + "." + NamingKt.clientName(sdkId) + "Client";
    }

    /** A shape's type name in aws-sdk-rust: smithy-rs Pascal-cases it (`ObjectCannedAcl`). */
    private static String rsSdkName(String modelName) {
        return pascal(modelName);
    }

    /** A shape's type name in aws-sdk-kotlin, by smithy-kotlin's own rule. */
    private String ktSdkName(ShapeId id) {
        return NamingKt.defaultName(model.expectShape(id), service);
    }

    /** A structure's SDK type, Rust. */
    private String rsSdkStruct(ShapeId id) {
        for (OperationShape op : operations) {
            if (output(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return rsCrate() + "::operation::" + snake(op.getId().getName()) + "::"
                        + rsSdkName(op.getId().getName()) + "Output";
            }
        }
        if (model.expectShape(id).hasTrait("smithy.api#error")) {
            return rsCrate() + "::types::error::" + rsSdkName(id.getName());
        }
        return rsCrate() + "::types::" + rsSdkName(id.getName());
    }

    /** A structure's SDK type, Kotlin (operation shapes are renamed Request/Response). */
    private String ktSdkStruct(ShapeId id) {
        for (OperationShape op : operations) {
            if (input(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return ktModel() + "." + NamingKt.capitalizedDefaultName(op) + "Request";
            }
            if (output(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return ktModel() + "." + NamingKt.capitalizedDefaultName(op) + "Response";
            }
        }
        return ktModel() + "." + ktSdkName(id);
    }

    private String effectOp(OperationShape op) {
        return snake(op.getId().getName());
    }

    /** What an operation answers instead of its output: the service's error or an [AwsError]. */
    private String errorUnion() {
        return settings.effect() + "Failure";
    }

    /** The service's one error struct. */
    private String errorStruct() {
        return settings.effect() + "Error";
    }

    /** The literal union of the service's error codes. */
    private String errorCodes() {
        return settings.effect() + "ErrorCode";
    }

    /**
     * The members an error shape carries besides its message — S3's
     * `InvalidObjectState` has a storage class and an access tier — which become
     * optional fields of the one error struct, set for that code only.
     */
    private Map<String, MemberShape> extraErrorMembers() {
        Map<String, MemberShape> out = new LinkedHashMap<>();
        for (StructureShape es : errorShapes.values()) {
            for (MemberShape m : members(es)) {
                if (m.getMemberName().equalsIgnoreCase("message")) continue;
                out.putIfAbsent(salvoField(m), m);
            }
        }
        return out;
    }

    /**
     * Every code the service may send for a modeled error, mapped to the model's
     * name: the model name itself, and the legacy `@awsQueryError` code where
     * the protocol is `awsQueryCompatible` (SQS's `QueueDoesNotExist` arrives
     * as `AWS.SimpleQueueService.NonExistentQueue` on one SDK). Both backends
     * normalize through this, so a code reads the same on either.
     */
    private Map<String, String> wireCodes() {
        Map<String, String> out = new TreeMap<>();
        for (Map.Entry<String, StructureShape> e : errorShapes.entrySet()) {
            e.getValue().findTrait("aws.protocols#awsQueryError").ifPresent(t -> {
                String code = t.toNode().expectObjectNode().expectStringMember("code").getValue();
                if (!code.equals(e.getKey())) out.put(code, e.getKey());
            });
        }
        return out;
    }

    // =============================================================== types ===

    private boolean isStreaming(MemberShape m) {
        return target(m).hasTrait(StreamingTrait.class);
    }

    /** Whether a structure holds a stream, which makes it a `linear struct`. */
    private boolean isLinear(ShapeId struct) {
        return streaming.containsKey(struct);
    }

    /** A member's Salvo type, without optionality. */
    private String salvoType(Shape t) {
        if (t.hasTrait(StreamingTrait.class)) return "InStream";
        if (t instanceof TimestampShape) return "Instant";
        if (t instanceof EnumShape e) return enums.get(e.getId());
        if (t instanceof StructureShape s) {
            return s.hasTrait("smithy.api#error") ? s.getId().getName() : structs.get(s.getId());
        }
        if (t instanceof ListShape l) return "List<" + salvoType(model.expectShape(l.getMember().getTarget())) + ">";
        if (t instanceof MapShape m) return "Map<Str, " + salvoType(model.expectShape(m.getValue().getTarget())) + ">";
        if (t instanceof StringShape) return "Str";
        if (t instanceof IntegerShape || t instanceof ShortShape || t instanceof ByteShape) return "Int";
        if (t instanceof LongShape) return "Long";
        if (t instanceof BooleanShape) return "Bool";
        if (t instanceof FloatShape) return "Float";
        if (t instanceof DoubleShape) return "Double";
        if (t instanceof BlobShape) return "Bytes";
        throw fail(t.getId().toString(), "has no Salvo type");
    }

    /**
     * Whether a member is present on every value: required, or defaulted — a
     * streaming member always is, since its `@default` is the empty body.
     */
    private boolean present(MemberShape m) {
        return m.hasTrait(RequiredTrait.class) || isStreaming(m) || defaultOf(m) != null;
    }

    /** A literal Salvo default for a `@default` member, or null (never for a stream). */
    private String defaultOf(MemberShape m) {
        if (isStreaming(m)) return null;
        return m.getTrait(DefaultTrait.class).map(d -> {
            var n = d.toNode();
            if (n.isNullNode()) return null;
            if (n.isBooleanNode()) return String.valueOf(n.expectBooleanNode().getValue());
            if (n.isNumberNode()) return n.expectNumberNode().getValue().toString();
            if (n.isStringNode()) return "\"" + n.expectStringNode().getValue().replace("\"", "\\\"") + "\"";
            throw fail(m.getId().toString(), "has a non-scalar @default, which this generator does not map yet");
        }).orElse(null);
    }

    private Shape target(MemberShape m) {
        return model.expectShape(m.getTarget());
    }

    // ================================================================ docs ===

    /**
     * The first paragraph of a shape's documentation, as plain text. Admonitions
     * (`<important>`, `<note>`) are not the summary, so they are skipped — S3's
     * `PutObject` opens with an end-of-support notice.
     */
    private static String docText(Shape s) {
        String html = s.getTrait(DocumentationTrait.class).map(DocumentationTrait::getValue).orElse("");
        html = html.replaceAll("(?s)<(important|note)>.*?</\\1>", "").trim();
        int end = html.indexOf("</p>");
        if (end >= 0) html = html.substring(0, end);
        String text = html.replaceAll("</?(p|li|ul|ol|br|dd|dt|dl)[^>]*>", " ").replaceAll("<[^>]+>", "")
                .replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
                .replace("&#39;", "'").replace("&amp;", "&")
                .replaceAll("\\s+", " ").replaceAll(" ([.,;:)])", "$1").trim();
        return text;
    }

    /** `// `-comment lines at [indent], wrapped to 80 columns. */
    private static String comment(String text, String indent) {
        if (text.isEmpty()) return "";
        StringBuilder out = new StringBuilder();
        StringBuilder line = new StringBuilder();
        int width = 80 - indent.length() - 3;
        for (String word : text.split(" ")) {
            if (line.length() > 0 && line.length() + 1 + word.length() > width) {
                out.append(indent).append("// ").append(line).append('\n');
                line.setLength(0);
            }
            if (line.length() > 0) line.append(' ');
            line.append(word);
        }
        if (line.length() > 0) out.append(indent).append("// ").append(line).append('\n');
        return out.toString();
    }

    private String header(String what) {
        return "// Generated by `salvo-client-codegen` from `" + settings.modelFile() + "`; do not\n"
                + "// edit — regenerate with `./gradlew generate` in `modules/aws/codegen`.\n";
    }

    // ======================================================= Salvo surface ===

    private String salvoSurface() {
        StringBuilder out = new StringBuilder();
        String title = service.getTrait(TitleTrait.class).map(TitleTrait::getValue).orElse(settings.effect());
        out.append(comment(title + ", as a Salvo effect: every operation takes a `Reply` and returns at once, "
                + "answering when the service does [platform-reply].", ""));
        out.append("//\n");
        out.append(comment(docText(service), ""));
        out.append("//\n");
        out.append(comment("The host implementation is `" + settings.module() + ".host`'s `Host"
                + settings.effect() + "`; `Fake" + settings.effect() + "` below is a recording double "
                + "that needs no SDK.", ""));
        out.append("//\n");
        out.append(header(""));
        out.append("\nimport aws\n");
        if (!streaming.isEmpty()) out.append("import stream\n");
        if (usesTime) out.append("import time.Instant\n");

        for (Map.Entry<ShapeId, String> e : enums.entrySet()) {
            out.append('\n').append(salvoEnum(model.expectShape(e.getKey(), EnumShape.class)));
        }
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            out.append('\n').append(salvoStruct(model.expectShape(s.getKey(), StructureShape.class), s.getValue()));
        }
        out.append(salvoErrors());

        // The effect.
        out.append('\n');
        out.append(comment(title + ". Every member hands its [reply] to the provider and returns at once; "
                + "the answer is the operation's output or a `Checked` [" + errorUnion() + "].", ""));
        out.append("export effect ").append(settings.effect()).append(" {\n");
        boolean first = true;
        for (OperationShape op : operations) {
            if (!first) out.append('\n');
            first = false;
            out.append(comment(docText(op), "    "));
            out.append("    ").append(effectSignature(op)).append('\n');
        }
        out.append("}\n");

        // The recording fake.
        String calls = settings.effect() + "Calls";
        out.append('\n');
        out.append(comment("What a [Fake" + settings.effect() + "] was asked, in order: one operation name per call.", ""));
        out.append("export effect ").append(calls).append(" {\n");
        out.append("    // The names of the operations called so far (`").append(effectOp(operations.get(0)))
                .append("`, …).\n");
        out.append("    fn calls() -> List<Str>\n}\n");
        out.append('\n');
        out.append(comment("A recording double for [" + settings.effect() + "]: every call is noted by operation "
                + "name — read them through [" + calls + "] — and answered with an empty output where the "
                + "operation's output has no required members, otherwise with `AwsError { code: \"NotStubbed\" }`. "
                + (streaming.isEmpty() ? "" : "A body it is handed is closed unread, and a body it answers with "
                        + "is empty, minted in the `Streams` in scope. ")
                + "A test that needs real answers implements [" + settings.effect() + "] itself.", ""));
        // A fake that hands out or takes in bodies does so through the
        // `Streams` in scope, as every other producer does [stream-layer].
        String deps = streaming.isEmpty() ? "" : " [Streams]";
        out.append("export handler Fake").append(settings.effect()).append("()").append(deps).append(" of ")
                .append(settings.effect()).append(", ").append(calls).append(" {\n");
        out.append("    recorded: Mut List<Str> = mut_list_of()\n");
        for (OperationShape op : operations) {
            out.append('\n');
            out.append("    ").append(effectSignature(op).replace("\n    =>", " =>")).append(" {\n");
            out.append("        recorded.add(\"").append(effectOp(op)).append("\")\n");
            if (input(op).map(i -> isLinear(i.getId())).orElse(false)) {
                // The body is not read; closing it is what discharges the input.
                // A missing length is refused as the host refuses it, so a test
                // against the fake fails where production would.
                String len = salvoField(lengthOf(input(op).get()));
                out.append("        let unsized = input.").append(len).append(" is None\n");
                out.append("        let closed = close(input)\n");
                out.append("        if closed is Err {\n            ignore(closed)\n        }\n");
                out.append("        if unsized {\n            reply.send(").append(missingLength(op)).append(")\n")
                        .append("            return None\n        }\n");
            }
            out.append("        reply.send(").append(fakeAnswer(op)).append(")\n");
            out.append("    }\n");
        }
        out.append("\n    fn calls() -> List<Str> {\n        return copy(recorded)\n    }\n}\n");
        return out.toString();
    }

    private String effectSignature(OperationShape op) {
        StringBuilder sig = new StringBuilder("fn ").append(effectOp(op)).append('(');
        List<String> moved = new ArrayList<>();
        input(op).ifPresent(s -> {
            sig.append("input: ").append(structs.get(s.getId())).append(", ");
            moved.add("!input");
        });
        sig.append("reply: Reply<").append(replyPayload(op)).append(">) -> None\n    => ");
        moved.add("!reply");
        sig.append(String.join(", ", moved));
        return sig.toString();
    }

    private String replyPayload(OperationShape op) {
        String out = output(op).map(s -> structs.get(s.getId())).orElse("None");
        return "Ok " + out + " | Err Checked<" + errorUnion() + ">";
    }

    /** The answer to a streamed input without its length, in Salvo. */
    private String missingLength(OperationShape op) {
        return "err(checked<" + errorUnion() + ">(AwsError { code: \"MissingContentLength\", message: \""
                + missingLengthText(op) + "\" }))";
    }

    private String missingLengthText(OperationShape op) {
        return settings.effect() + " " + op.getId().getName() + " streams its body, so the input needs "
                + salvoField(lengthOf(input(op).get())) + ": the body's length in bytes";
    }

    private String fakeAnswer(OperationShape op) {
        Optional<StructureShape> out = output(op);
        if (out.isEmpty()) return "ok(None)";
        // A stream is present on every value, and the empty one is an empty body.
        boolean stubbable = members(out.get()).stream()
                .noneMatch(m -> present(m) && !isStreaming(m));
        if (stubbable) {
            List<String> fields = new ArrayList<>();
            for (MemberShape m : members(out.get())) {
                if (isStreaming(m)) fields.add(salvoField(m) + ": from_bytes(bytes_of())");
            }
            String body = fields.isEmpty() ? " {}" : " { " + String.join(", ", fields) + " }";
            return "ok(" + structs.get(out.get().getId()) + body + ")";
        }
        return "err(checked<" + errorUnion() + ">(AwsError { code: \"NotStubbed\", message: \"Fake"
                + settings.effect() + " cannot answer " + effectOp(op) + "\" }))";
    }

    /**
     * The service's errors as data: named code groups (`errorGroups` in the
     * settings), the code union — every modeled error of the selected
     * operations, and `Other Str` for the rest (throttling, access denied, a
     * code this model predates) — the one error struct, and the failure union
     * with [AwsError] for a call that got no response.
     */
    private String salvoErrors() {
        StringBuilder out = new StringBuilder("\n");
        List<String> arms = new ArrayList<>();
        Set<String> grouped = new LinkedHashSet<>();
        for (Map.Entry<String, String> g : settings.errorGroups().entrySet()) {
            List<String> codes = new ArrayList<>();
            for (String name : errorNames) {
                if (name.startsWith(g.getValue())) codes.add("\"" + name + "\"");
            }
            if (codes.isEmpty()) continue;
            grouped.addAll(codes);
            out.append(comment("The " + errorStruct() + " codes starting `" + g.getValue() + "`, as one group: `if "
                    + "e.code is " + g.getKey() + "` asks whether a failure was one of them.", ""));
            out.append("export type ").append(g.getKey()).append(" = ").append(String.join("\n    | ", codes))
                    .append("\n\n");
            arms.add(g.getKey());
        }
        for (String name : errorNames) {
            if (!grouped.contains("\"" + name + "\"")) arms.add("\"" + name + "\"");
        }
        arms.add("Other Str");
        out.append(comment("A code " + settings.effect() + " answers with: one arm per error the model names for "
                + "these operations, and `Other` for anything else the service sends — throttling, an "
                + "authorization failure, a code newer than this model. A legacy wire code is normalized to the "
                + "model's name, so a code reads the same on every backend.", ""));
        out.append("export type ").append(errorCodes()).append(" = ").append(String.join("\n    | ", arms))
                .append("\n\n");
        out.append(comment("The service answered with an error: what it said, and the HTTP status it said it "
                + "with.", ""));
        out.append("export struct ").append(errorStruct()).append(" {\n");
        out.append("    code: ").append(errorCodes()).append(",\n");
        out.append("    message: Str,\n");
        out.append("    // The HTTP status of the response.\n");
        out.append("    status: Int,\n");
        out.append("    // The service's id for the request, when it sent one: what AWS support asks for.\n");
        Map<String, MemberShape> extras = extraErrorMembers();
        out.append("    request_id: Str? = None").append(extras.isEmpty() ? "\n" : ",\n");
        int i = 0;
        for (Map.Entry<String, MemberShape> x : extras.entrySet()) {
            String owner = x.getValue().getContainer().getName();
            out.append(comment("Set for `" + owner + "` only. " + docText(x.getValue()), "    "));
            out.append("    ").append(x.getKey()).append(": ").append(salvoType(target(x.getValue())))
                    .append("? = None").append(++i < extras.size() ? ",\n" : "\n");
        }
        out.append("}\n\n");
        out.append(comment("Everything an operation of [" + settings.effect() + "] can answer with instead of its "
                + "output: the service's error, or an [AwsError] when there was no answer to read.", ""));
        out.append("export type ").append(errorUnion()).append(" = ").append(errorStruct()).append(" | AwsError\n");
        return out.toString();
    }

    private String salvoEnum(EnumShape e) {
        String name = enums.get(e.getId());
        StringBuilder out = new StringBuilder();
        out.append(comment(docText(e).isEmpty() ? name + ", one literal per value the model names." : docText(e), ""));
        out.append("//\n");
        out.append(comment("Open, as Smithy enums are: a value this model does not name arrives as the `Other` arm "
                + "[type-literal]. At run time a plain string, the value as it is on the wire.", ""));
        List<String> arms = new ArrayList<>();
        for (String v : e.getEnumValues().values()) arms.add("\"" + v.replace("\"", "\\\"") + "\"");
        arms.add("Other Str");
        out.append("export type ").append(name).append(" = ").append(String.join("\n    | ", arms)).append('\n');
        return out.toString();
    }

    private String salvoStruct(StructureShape s, String name) {
        StringBuilder out = new StringBuilder();
        out.append(comment(docText(s), ""));
        List<MemberShape> members = new ArrayList<>(members(s));
        List<String> omitted = new ArrayList<>();
        for (MemberShape m : s.getAllMembers().values()) {
            if (!members.contains(m)) omitted.add("`" + salvoField(m) + "`");
        }
        if (!omitted.isEmpty()) {
            if (!docText(s).isEmpty()) out.append("//\n");
            out.append(comment("Not mapped: " + String.join(", ", omitted) + " — the SDKs customize "
                    + (omitted.size() == 1 ? "it" : "them") + " away from the model (`omitMembers` in "
                    + "smithy-build.json).", ""));
        }
        if (members.isEmpty()) {
            out.append("export struct ").append(name).append(" {}\n");
            return out.toString();
        }
        boolean linear = isLinear(s.getId());
        if (linear) {
            if (!docText(s).isEmpty()) out.append("//\n");
            out.append(comment("Linear, because it holds a stream: take the stream out (`let {"
                    + salvoField(streaming.get(s.getId())) + "} = value`) and close it when done, or give "
                    + "the whole value up with [close] [linear-group].", ""));
            if (inputs.contains(s.getId())) {
                String len = salvoField(lengthOf(s));
                out.append("//\n");
                out.append(comment("The " + salvoField(streaming.get(s.getId())) + " is streamed as it is read, "
                        + "so [" + len + "] must be its length in bytes — for a file, `metadata(path)`'s `size`. "
                        + "Without it the call answers `AwsError { code: \"MissingContentLength\" }` and sends "
                        + "nothing; a stream shorter or longer than it answers `AwsError { code: \"StreamFailed\" }`.",
                        ""));
            }
        }
        out.append(linear ? "export linear struct " : "export struct ").append(name).append(" {\n");
        for (int i = 0; i < members.size(); i++) {
            MemberShape m = members.get(i);
            out.append(comment(docText(m), "    "));
            String ty = salvoType(target(m));
            out.append("    ").append(salvoField(m)).append(": ");
            String def = defaultOf(m);
            if (m.hasTrait(RequiredTrait.class) || isStreaming(m)) {
                out.append(ty);
            } else if (def != null) {
                out.append(ty).append(" = ").append(def);
            } else {
                out.append(ty).append("? = None");
            }
            out.append(i + 1 < members.size() ? ",\n" : "\n");
        }
        out.append("}\n");
        if (linear) {
            String field = salvoField(streaming.get(s.getId()));
            out.append('\n');
            out.append(comment("Gives up [value] without reading its " + field + ": the stream is closed and "
                    + "the other members are dropped. The discharger of [" + name + "] [linear-group].", ""));
            out.append("export fn close(value: ").append(name)
                    .append(") [Streams] -> Ok None | Err Checked<StreamError> => !value {\n");
            out.append("    return close(value.").append(field).append(")\n}\n");
        }
        return out.toString();
    }

    private String salvoHost() {
        StringBuilder out = new StringBuilder();
        out.append(comment("The host's " + service.getTrait(TitleTrait.class).map(TitleTrait::getValue)
                .orElse(settings.effect()) + ": [Host" + settings.effect() + "] implements `" + settings.module()
                + "`'s [" + settings.effect() + "] over aws-sdk-kotlin and aws-sdk-rust.", ""));
        out.append("//\n");
        out.append(comment("Its own module, as `fs.host` is: a program that fakes the service never reaches "
                + "the host glue, so it builds without the SDK.", ""));
        out.append("//\n");
        out.append(header(""));
        out.append("\nimport aws\nimport ").append(settings.module()).append("\n\n");
        out.append(comment("[" + settings.effect() + "] through the platform's AWS SDK, configured by [config]: "
                + "each call runs on the SDK's own asynchronous machinery (coroutines on the JVM, tokio on "
                + "Rust) and completes its reply from there, so no Salvo worker waits. `threadsafe`: the SDK "
                + "client is safe to share, so the instance is used from every pool with no lock.", ""));
        out.append("export threadsafe platform handler Host").append(settings.effect())
                .append("(config: AwsConfig) of ").append(settings.effect()).append('\n');
        return out.toString();
    }

    // ============================================================ Rust glue ===

    private String rsSalvoModule() {
        return "crate::" + settings.module().replace('.', '_');
    }

    /** A Salvo struct or enum name as the Rust emitter spells it (dot-names flattened). */
    private static String rsType(String salvoName) {
        return salvoName.replace(".", "");
    }

    private String rsErrorUnionType() {
        return "Union2<" + errorStruct() + ", AwsError>";
    }

    /** Salvo → SDK, from a reference expression [ref] to the Salvo value. */
    private String rsToSdk(Shape t, String ref) {
        // [type-literal] An enum is its wire string in Salvo, on both sides.
        if (t instanceof EnumShape e) {
            return rsCrate() + "::types::" + rsSdkName(e.getId().getName()) + "::from(" + ref + ".as_str())";
        }
        if (t instanceof StructureShape s) return "to_sdk_" + snake(structs.get(s.getId())) + "(" + ref + ")";
        if (t instanceof ListShape l) {
            return ref + ".iter().map(|e| " + rsToSdk(target(l.getMember()), "e") + ").collect::<Vec<_>>()";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape
                    ? rsCrate() + "::types::" + rsSdkName(key.getId().getName()) + "::from(k.as_str())"
                    : "k.clone()";
            return ref + ".iter().map(|(k, v)| (" + k + ", " + rsToSdk(target(m.getValue()), "v")
                    + ")).collect::<std::collections::HashMap<_, _>>()";
        }
        if (t instanceof StringShape) return ref + ".clone()";
        if (t instanceof BlobShape) return rsCrate() + "::primitives::Blob::new(" + ref + ".clone())";
        if (t instanceof TimestampShape) return "salvo_date_time(" + ref + ")";
        return "*" + ref;
    }

    /** SDK → Salvo, from a reference expression [ref] to the SDK value. */
    private String rsFromSdk(Shape t, String ref) {
        if (t instanceof EnumShape) return ref + ".as_str().to_string()";
        if (t instanceof StructureShape s) return "from_sdk_" + snake(salvoType(s)) + "(" + ref + ")";
        if (t instanceof ListShape l) {
            return ref + ".iter().map(|e| " + rsFromSdk(target(l.getMember()), "e") + ").collect::<Vec<_>>()";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? "k.as_str().to_string()" : "k.clone()";
            return "{ let mut __es: Vec<_> = " + ref + ".iter().map(|(k, v)| (" + k + ", "
                    + rsFromSdk(target(m.getValue()), "v") + ")).collect(); "
                    + "__es.sort_by(|a, b| a.0.cmp(&b.0)); "
                    + "crate::platform_core_map::canonical_map(__es) }";
        }
        if (t instanceof StringShape) return ref + ".to_string()";
        if (t instanceof BlobShape) return ref + ".as_ref().to_vec()";
        if (t instanceof TimestampShape) return "salvo_instant(" + ref + ")";
        return "*" + ref;
    }

    private static boolean isPrimitive(Shape t) {
        return t instanceof IntegerShape || t instanceof ShortShape || t instanceof ByteShape
                || t instanceof LongShape || t instanceof BooleanShape || t instanceof FloatShape
                || t instanceof DoubleShape;
    }

    /** The Salvo value of one member, read from the SDK value [src] (a reference). */
    private String rsReadField(MemberShape m, String src) {
        Shape t = target(m);
        String acc = src + "." + rsAccessor(m) + "()";
        boolean present = present(m);
        if (t instanceof ListShape) {
            // smithy-rs answers `&[T]` for a list, empty when absent.
            if (present) return rsFromSdk(t, acc);
            return "{ let __s = " + acc + "; if __s.is_empty() { None } else { Some(" + rsFromSdk(t, "__s") + ") } }";
        }
        if (isPrimitive(t)) {
            return present ? rsFromSdk(t, "(&" + acc + ")") : acc + ".map(|x| " + rsFromSdk(t, "(&x)") + ")";
        }
        return present ? rsFromSdk(t, acc) : acc + ".map(|x| " + rsFromSdk(t, "x") + ")";
    }

    /** The `set_x(...)` argument for one member, from the Salvo struct [src] (a place). */
    private String rsSetArg(MemberShape m, String src) {
        Shape t = target(m);
        String field = src + "." + rsIdent(salvoField(m));
        if (m.hasTrait(RequiredTrait.class) || defaultOf(m) != null) {
            return "Some(" + rsToSdk(t, "(&" + field + ")") + ")";
        }
        return field + ".as_ref().map(|x| " + rsToSdk(t, "x") + ")";
    }

    private String rustGlue() {
        String crate = rsCrate();
        String effect = settings.effect();
        StringBuilder out = new StringBuilder();
        out.append("// Host implementation of `").append(settings.module()).append(".host`'s `Host").append(effect)
                .append("`,\n// over ").append(crate).append(" [platform-reply].\n//\n");
        out.append(header(""));
        out.append("\n#![allow(unused_imports, dead_code, deprecated)]\n");
        out.append("\nuse crate::aws::*;\nuse ").append(rsSalvoModule()).append("::*;\nuse ")
                .append(rsSalvoModule()).append("_host::*;\nuse crate::unions::*;\nuse crate::core_checked::Checked;\n");
        out.append("use ").append(crate).append("::error::ProvideErrorMetadata;\n\n");

        out.append("// `threadsafe platform handler Host").append(effect).append("`: the SDK client is cheap to\n")
                .append("// clone and safe to share, and each call runs as a task on the handler's own\n")
                .append("// tokio runtime, completing its reply from there.\n");
        out.append("pub struct Host").append(effect).append(" {\n    rt: tokio::runtime::Runtime,\n    client: ")
                .append(crate).append("::Client,\n}\n\n");
        out.append("impl Host").append(effect).append(" {\n    pub fn new(config: AwsConfig) -> Self {\n")
                .append("        let rt = tokio::runtime::Builder::new_multi_thread()\n")
                .append("            .enable_all()\n            .build()\n")
                .append("            .expect(\"a tokio runtime for the AWS SDK\");\n")
                .append("        let shared = rt.block_on(salvo_aws_config(&config));\n")
                .append(settings.forcePathStyle()
                        ? "        let client = " + crate + "::Client::from_conf(\n"
                                + "            " + crate + "::config::Builder::from(&shared)\n"
                                + "                .force_path_style(config.endpoint.is_some())\n"
                                + "                .build(),\n        );\n"
                        : "        let client = " + crate + "::Client::new(&shared);\n")
                .append("        Self { rt, client }\n    }\n}\n\n");
        out.append(rustConfigLoader());

        out.append("impl ").append(rsSalvoModule()).append("::").append(effect).append("PlatformSync for Host")
                .append(effect).append(" {\n");
        boolean first = true;
        for (OperationShape op : operations) {
            if (!first) out.append('\n');
            first = false;
            out.append(rustMember(op));
        }
        out.append("}\n");

        // Conversions.
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            StructureShape shape = model.expectShape(s.getKey(), StructureShape.class);
            if (!inputs.contains(s.getKey())) out.append('\n').append(rustFromSdk(shape, s.getValue()));
            if (!outputs.contains(s.getKey()) && !inputs.contains(s.getKey())) {
                out.append('\n').append(rustToSdk(shape, s.getValue()));
            }
        }
        out.append(rustFailure());
        out.append(rustExpandHome());
        if (usesTime) out.append(rustTimeHelpers());
        if (!streaming.isEmpty()) out.append(rustStreamHelpers());
        return out.toString();
    }

    /**
     * The member carrying a streaming input's length — the one bound to the
     * `Content-Length` header. A streamed request body must say its length
     * (user decision 2026-09-30: no silent buffering), so an input with a
     * stream and no such member is refused here rather than generated.
     */
    private MemberShape lengthOf(StructureShape s) {
        for (MemberShape m : members(s)) {
            if (m.getTrait(HttpHeaderTrait.class).map(h -> h.getValue().equalsIgnoreCase("Content-Length")).orElse(false)
                    && (target(m) instanceof LongShape || target(m) instanceof IntegerShape)) {
                return m;
            }
        }
        throw fail(s.getId().toString(), "streams a request body but has no Content-Length member to say its length");
    }

    /** The streaming member of an operation's input or output, or null. */
    private MemberShape streamOf(Optional<StructureShape> s) {
        return s.map(x -> streaming.get(x.getId())).orElse(null);
    }

    /** How a stream the glue registers describes itself: `S3 GetObject body`. */
    private String bodySource(OperationShape op) {
        return settings.effect() + " " + op.getId().getName() + " " + snake(streamOf(output(op)).getMemberName());
    }

    private String rustMember(OperationShape op) {
        String crate = rsCrate();
        String name = effectOp(op);
        MemberShape inStream = streamOf(input(op));
        MemberShape outStream = streamOf(output(op));
        String okType = output(op).map(s -> rsType(structs.get(s.getId()))).orElse("()");
        String answerType = "Union2<" + okType + ", Checked<" + rsErrorUnionType() + ">>";
        StringBuilder out = new StringBuilder();
        out.append("    fn ").append(name).append("(&self, ");
        if (input(op).isPresent()) {
            out.append("input: ").append(rsType(structs.get(input(op).get().getId()))).append(", ");
        }
        out.append("reply: crate::scheduler::SalvoReply) {\n");
        out.append("        let reply = reply.hosted();\n        let client = self.client.clone();\n");
        if (inStream != null) {
            // [stream-table] The request body is taken out of the host's table
            // now — the token was given up with the input — and streamed from
            // a blocking thread once the task runs. A handle another table
            // minted traps here [stream-provider].
            out.append("        let body = crate::hoststreams::salvo_stream_take_in(input.")
                    .append(rsIdent(salvoField(inStream))).append(".handle);\n");
            out.append("        let length = input.").append(rsIdent(salvoField(lengthOf(input(op).get()))))
                    .append(";\n");
        }
        if (outStream != null) out.append("        let rt = self.rt.handle().clone();\n");
        out.append("        let call = client.").append(name).append("()");
        input(op).ifPresent(in -> {
            for (MemberShape m : members(in)) {
                if (isStreaming(m)) continue;
                out.append("\n            .set_").append(snake(m.getMemberName())).append('(')
                        .append(rsSetArg(m, "input")).append(')');
            }
        });
        out.append(";\n");
        out.append("        self.rt.spawn(async move {\n");
        String obj = opObject(op);
        String failWith = obj + "::err(Checked { value: " + rsFailure("AwsError") + "(AwsError { code: ";
        if (inStream != null) {
            // No length, no request: the body is released unread (user
            // decision 2026-09-30 — never buffer to find it out).
            out.append("            let length = match length {\n")
                    .append("                Some(n) if n >= 0 => n,\n")
                    .append("                _ => {\n                    drop(body);\n")
                    .append("                    let failed: ").append(answerType).append(" = ").append(failWith)
                    .append("\"MissingContentLength\".to_string(), message: \"").append(missingLengthText(op))
                    .append("\".to_string() }) });\n")
                    .append("                    reply.send(failed);\n                    return;\n                }\n")
                    .append("            };\n");
            out.append("            let (upload, problem) = salvo_upload(body, length);\n");
            out.append("            let call = call.set_").append(snake(inStream.getMemberName())).append("(Some(")
                    .append(crate).append("::primitives::ByteStream::from_body_1_x(upload)));\n");
        }
        out.append("            let answer: ").append(answerType).append(" = match call.send().await {\n");
        if (outStream != null) {
            out.append("                Ok(out) => ").append(obj).append("::ok(from_sdk_").append(snake(structs.get(output(op).get().getId())))
                    .append("(out, &rt)),\n");
        } else {
            String ok = output(op).map(s -> "from_sdk_" + snake(structs.get(s.getId())) + "(&out)").orElse("()");
            out.append("                Ok(out) => { let _ = &out; ").append(obj).append("::ok(").append(ok).append(") }\n");
        }
        out.append("                Err(e) => ").append(obj).append("::err(salvo_failure(e, ").append(rustExtra(op)).append(")),\n");
        out.append("            };\n");
        if (inStream != null) {
            // A body that failed, or did not match its length, is the answer
            // whatever the service said about the bytes it did get.
            out.append("            let problem = problem.lock().unwrap().take();\n")
                    .append("            let answer: ").append(answerType).append(" = match problem {\n")
                    .append("                Some(message) => ").append(failWith)
                    .append("\"StreamFailed\".to_string(), message }) }),\n")
                    .append("                None => answer,\n            };\n");
        }
        out.append("            reply.send(answer);\n        });\n    }\n");
        return out.toString();
    }

    private String rustFromSdk(StructureShape s, String name) {
        MemberShape body = streaming.get(s.getId());
        StringBuilder out = new StringBuilder();
        if (body != null) {
            // The answer is taken by value: its body moves into the host's
            // stream table, read later by whichever `Streams` the program binds.
            OperationShape op = operations.stream()
                    .filter(o -> output(o).map(Shape::getId).filter(s.getId()::equals).isPresent())
                    .findFirst().orElseThrow();
            out.append("fn from_sdk_").append(snake(name)).append("(v: ").append(rsSdkStruct(s.getId()))
                    .append(", rt: &tokio::runtime::Handle) -> ").append(rsType(name)).append(" {\n    ")
                    .append(rsType(name)).append(" {\n");
            for (MemberShape m : members(s)) {
                if (m.equals(body)) continue;
                out.append("        ").append(rsIdent(salvoField(m))).append(": ").append(rsReadField(m, "v")).append(",\n");
            }
            // Last: moving the body out of `v` ends every borrow above.
            out.append("        ").append(rsIdent(salvoField(body))).append(": crate::stream::InStream {\n")
                    .append("            handle: salvo_register_body(\"").append(bodySource(op)).append("\", v.")
                    .append(rsAccessor(body)).append(", rt),\n        },\n");
            out.append("    }\n}\n");
            return out.toString();
        }
        out.append("fn from_sdk_").append(snake(name)).append("(v: &").append(rsSdkStruct(s.getId())).append(") -> ")
                .append(rsType(name)).append(" {\n    ").append(rsType(name)).append(" {");
        List<MemberShape> members = new ArrayList<>(members(s));
        if (members.isEmpty()) return out.append("}\n}\n").toString();
        out.append('\n');
        for (MemberShape m : members) {
            out.append("        ").append(rsIdent(salvoField(m))).append(": ").append(rsReadField(m, "v")).append(",\n");
        }
        out.append("    }\n}\n");
        return out.toString();
    }

    private String rustToSdk(StructureShape s, String name) {
        String sdk = rsSdkStruct(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("fn to_sdk_").append(snake(name)).append("(v: &").append(rsType(name)).append(") -> ").append(sdk)
                .append(" {\n    let b = ").append(sdk).append("::builder()");
        boolean fallible = false;
        for (MemberShape m : members(s)) {
            out.append("\n        .set_").append(snake(m.getMemberName())).append('(').append(rsSetArg(m, "v")).append(')');
            fallible |= m.hasTrait(RequiredTrait.class);
        }
        out.append(";\n    b.build()");
        if (fallible) out.append(".expect(\"every required member of ").append(name).append(" is set\")");
        out.append("\n}\n");
        return out.toString();
    }

    /**
     * The one error conversion, generic over every operation's error type: a
     * service error becomes the service's error struct — the code normalized to
     * the model's name, the message, the HTTP status, the request id — and
     * anything else (no response: a timeout, a credentials or dispatch failure,
     * a response that did not parse) an [AwsError]. [extra] fills the fields
     * one error carries (S3's `InvalidObjectState`).
     */
    private String rustFailure() {
        String crate = rsCrate();
        String es = errorStruct();
        StringBuilder out = new StringBuilder();
        out.append("\n/// A code as the model names it: the shape id's name (`ns#Name`), and a legacy\n")
                .append("/// `@awsQueryError` code mapped to the model's.\n");
        out.append("fn salvo_code(code: &str) -> String {\n    let code = code.rsplit('#').next().unwrap_or(code);\n")
                .append("    match code {\n");
        for (Map.Entry<String, String> c : wireCodes().entrySet()) {
            out.append("        \"").append(c.getKey()).append("\" => \"").append(c.getValue()).append("\",\n");
        }
        out.append("        other => other,\n    }\n    .to_string()\n}\n\n");
        out.append("fn salvo_failure<E>(\n    e: ").append(crate).append("::error::SdkError<E, ").append(crate)
                .append("::config::http::HttpResponse>,\n    extra: impl FnOnce(&E, &mut ").append(es)
                .append("),\n) -> Checked<").append(rsErrorUnionType()).append(">\nwhere\n")
                .append("    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,\n{\n")
                .append("    use ").append(crate).append("::operation::RequestId;\n")
                .append("    let text = format!(\"{}\", ").append(crate).append("::error::DisplayErrorContext(&e));\n")
                .append("    let request_id = e.request_id().map(|r| r.to_string());\n")
                .append("    let value = match &e {\n")
                .append("        ").append(crate).append("::error::SdkError::ServiceError(ctx) => {\n")
                .append("            let err = ctx.err();\n")
                .append("            let mut out = ").append(es).append(" {\n")
                .append("                code: salvo_code(err.code().unwrap_or(\"Unknown\")),\n")
                .append("                message: err.message().unwrap_or(\"\").to_string(),\n")
                .append("                status: ctx.raw().status().as_u16() as i32,\n")
                .append("                request_id,\n");
        for (String f : extraErrorMembers().keySet()) out.append("                ").append(rsIdent(f)).append(": None,\n");
        out.append("            };\n            extra(err, &mut out);\n            ").append(rsFailure(es)).append("(out)\n        }\n")
                .append("        other => ").append(rsFailure("AwsError")).append("(AwsError {\n")
                .append("            code: match other {\n")
                .append("                ").append(crate).append("::error::SdkError::TimeoutError(_) => \"TimeoutError\",\n")
                .append("                ").append(crate).append("::error::SdkError::DispatchFailure(_) => \"DispatchFailure\",\n")
                .append("                ").append(crate).append("::error::SdkError::ResponseError(_) => \"ResponseError\",\n")
                .append("                _ => \"ConstructionFailure\",\n            }\n            .to_string(),\n")
                .append("            message: text,\n        }),\n    };\n    Checked { value }\n}\n");
        return out.toString();
    }

    /** The `extra` closure for one operation: the fields its errors carry. */
    private String rustExtra(OperationShape op) {
        String crate = rsCrate();
        String errEnum = crate + "::operation::" + effectOp(op) + "::" + rsSdkName(op.getId().getName()) + "Error";
        StringBuilder arms = new StringBuilder();
        for (ShapeId err : op.getErrors(service)) {
            StructureShape es = errorShapes.get(err.getName());
            List<MemberShape> extra = members(es).stream()
                    .filter(m -> !m.getMemberName().equalsIgnoreCase("message")).toList();
            if (extra.isEmpty()) continue;
            arms.append("                if let ").append(errEnum).append("::").append(rsSdkName(err.getName()))
                    .append("(x) = err {\n");
            for (MemberShape m : extra) {
                arms.append("                    out.").append(rsIdent(salvoField(m))).append(" = ")
                        .append(rsReadField(m, "x")).append(";\n");
            }
            arms.append("                }\n");
        }
        if (arms.length() == 0) return "|_, _| {}";
        return "|err, out| {\n" + arms + "            }";
    }

    private String rustConfigLoader() {
        return """
                // The SDK configuration an `AwsConfig` describes: the region, the credentials
                // it names, and the endpoint override when there is one.
                #[allow(deprecated)]
                async fn salvo_aws_config(config: &AwsConfig) -> aws_config::SdkConfig {
                    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
                        .region(aws_config::Region::new(config.region.code.clone()));
                    match &config.credentials {
                        Union3::U1(p) => {
                            use aws_config::profile::profile_file::{ProfileFileKind, ProfileFiles};
                            let files = ProfileFiles::builder()
                                .with_file(ProfileFileKind::Credentials, salvo_expand_home(&p.path))
                                .build();
                            let provider = aws_config::profile::ProfileFileCredentialsProvider::builder()
                                .profile_files(files)
                                .profile_name(p.profile.clone())
                                .build();
                            loader = loader.credentials_provider(provider);
                        }
                        Union3::U2(_) => {
                            loader = loader.credentials_provider(
                                aws_config::environment::EnvironmentVariableCredentialsProvider::new(),
                            );
                        }
                        Union3::U3(_) => {}
                    }
                    if let Some(endpoint) = &config.endpoint {
                        loader = loader.endpoint_url(endpoint.clone());
                    }
                    loader.load().await
                }

                """;
    }

    private String rustTimeHelpers() {
        String dt = rsCrate() + "::primitives::DateTime";
        return """

                /// A Smithy timestamp as a `time.Instant`: nanoseconds since the epoch,
                /// saturating outside `Instant`'s range (1678–2262).
                fn salvo_instant(t: &%1$s) -> crate::time::Instant {
                    let nanos = t.as_nanos().clamp(i64::MIN as i128, i64::MAX as i128) as i64;
                    crate::time::Instant { nanos }
                }

                /// A `time.Instant` as a Smithy timestamp.
                fn salvo_date_time(at: &crate::time::Instant) -> %1$s {
                    %1$s::from_nanos(at.nanos as i128).expect("an i64 of nanoseconds is a valid timestamp")
                }
                """.formatted(dt);
    }

    private String rustStreamHelpers() {
        String bs = rsCrate() + "::primitives::ByteStream";
        return """

                /// [stream-table] Registers a response body in the host's stream table,
                /// answering its handle. The table reads synchronously, from a Salvo worker
                /// or the stream host's reader thread — never from a task on the SDK's
                /// runtime — so each read blocks on the next chunk through [rt].
                fn salvo_register_body(source: &str, body: %1$s, rt: &tokio::runtime::Handle) -> i64 {
                    let reader = SalvoBody { rt: rt.clone(), body, chunk: Vec::new(), at: 0 };
                    crate::hoststreams::salvo_stream_register_in(source.to_string(), Box::new(reader), 0)
                }

                /// A response body as `std::io::Read`.
                struct SalvoBody {
                    rt: tokio::runtime::Handle,
                    body: %1$s,
                    chunk: Vec<u8>,
                    at: usize,
                }

                impl std::io::Read for SalvoBody {
                    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                        while self.at == self.chunk.len() {
                            match self.rt.block_on(self.body.try_next()) {
                                Ok(Some(bytes)) => {
                                    self.chunk = bytes.to_vec();
                                    self.at = 0;
                                }
                                Ok(None) => return Ok(0),
                                Err(e) => return Err(std::io::Error::other(e)),
                            }
                        }
                        let n = buf.len().min(self.chunk.len() - self.at);
                        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
                        self.at += n;
                        Ok(n)
                    }
                }

                /// [stream-table] A request body streamed out of the host's table: a
                /// blocking thread reads it in chunks into a small channel the SDK's
                /// body drains, so memory stays at a few chunks whatever the size. The
                /// body is exactly `length` bytes — the SDKs sign and checksum a body of
                /// known length — and a stream that ends early or runs on past it is a
                /// failure, recorded in the answer's slot and sent to the SDK as an error
                /// frame. Not retryable: a stream is read once.
                fn salvo_upload(
                    body: std::sync::Arc<std::sync::Mutex<crate::hoststreams::SalvoIn>>,
                    length: i64,
                ) -> (SalvoUpload, std::sync::Arc<std::sync::Mutex<Option<String>>>) {
                    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<bytes::Bytes>>(4);
                    let problem = std::sync::Arc::new(std::sync::Mutex::new(None));
                    let record = problem.clone();
                    tokio::task::spawn_blocking(move || {
                        let mut stream = body.lock().unwrap();
                        let fault = |source: &str, f: crate::hoststreams::SalvoFault| match f {
                            crate::hoststreams::SalvoFault::Utf8 => format!("{source}: not valid UTF-8"),
                            crate::hoststreams::SalvoFault::Failed(m) => format!("{source}: {m}"),
                        };
                        let mut left = length as u64;
                        let failed: Option<String> = loop {
                            let mut buf = Vec::new();
                            if left == 0 {
                                break None;
                            }
                            match stream.read_up_to(&mut buf, left.min(65536) as usize) {
                                Ok(0) => {
                                    break Some(format!(
                                        "{}: ended after {} of the content_length of {length} bytes",
                                        stream.source,
                                        length as u64 - left
                                    ))
                                }
                                Ok(n) => {
                                    left -= n as u64;
                                    if left == 0 {
                                        // One byte past the end tells a stream longer than its
                                        // length — checked *before* the last chunk goes, so the
                                        // request fails rather than storing a truncated object.
                                        let mut extra = Vec::new();
                                        match stream.read_up_to(&mut extra, 1) {
                                            Ok(0) => {}
                                            Ok(_) => {
                                                break Some(format!(
                                                    "{}: longer than the content_length of {length} bytes",
                                                    stream.source
                                                ))
                                            }
                                            Err(f) => break Some(fault(&stream.source, f)),
                                        }
                                    }
                                    if tx.blocking_send(Ok(bytes::Bytes::from(buf))).is_err() {
                                        // The SDK gave up on the body; its error is the answer.
                                        break None;
                                    }
                                }
                                Err(f) => break Some(fault(&stream.source, f)),
                            }
                        };
                        if let Some(message) = failed {
                            *record.lock().unwrap() = Some(message.clone());
                            let _ = tx.blocking_send(Err(std::io::Error::other(message)));
                        }
                    });
                    (SalvoUpload { rx, length: length as u64 }, problem)
                }

                /// The SDK's side of [salvo_upload]: an `http_body::Body` of exactly
                /// `length` bytes over the channel.
                struct SalvoUpload {
                    rx: tokio::sync::mpsc::Receiver<std::io::Result<bytes::Bytes>>,
                    length: u64,
                }

                impl http_body::Body for SalvoUpload {
                    type Data = bytes::Bytes;
                    type Error = std::io::Error;

                    fn poll_frame(
                        mut self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Option<Result<http_body::Frame<bytes::Bytes>, std::io::Error>>> {
                        self.rx.poll_recv(cx).map(|next| next.map(|chunk| chunk.map(http_body::Frame::data)))
                    }

                    fn size_hint(&self) -> http_body::SizeHint {
                        http_body::SizeHint::with_exact(self.length)
                    }
                }
                """.formatted(bs);
    }

    private String rustExpandHome() {
        return """

                /// `~/` at the front of a path is the home directory, as a shell would read it.
                fn salvo_expand_home(path: &str) -> String {
                    match (path.strip_prefix("~/"), std::env::var("HOME")) {
                        (Some(rest), Ok(home)) => format!("{home}/{rest}"),
                        _ => path.to_string(),
                    }
                }
                """;
    }

    // ========================================================== Kotlin glue ===

    private static String stars(int n) {
        return "<" + String.join(", ", java.util.Collections.nCopies(n, "*")) + ">";
    }

    /** Salvo → SDK, from a non-null expression [v]; [d] numbers nested lambda parameters. */
    private String ktToSdk(Shape t, String v, int d) {
        // [type-literal] An enum is its wire string in Salvo, on both sides.
        if (t instanceof EnumShape e) return ktModel() + "." + ktSdkName(e.getId()) + ".fromValue(" + v + ")";
        if (t instanceof StructureShape s) return "toSdk" + structs.get(s.getId()) + "(" + v + ")";
        if (t instanceof ListShape l) {
            return v + ".map { e" + d + " -> " + ktToSdk(target(l.getMember()), "e" + d, d + 1) + " }";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? ktModel() + "." + ktSdkName(key.getId()) + ".fromValue(k" + d + ")" : "k" + d;
            return v + ".associate { (k" + d + ", v" + d + ") -> " + k + " to "
                    + ktToSdk(target(m.getValue()), "v" + d, d + 1) + " }";
        }
        if (t instanceof BlobShape) return v + ".toByteArray()";
        if (t instanceof TimestampShape) return "salvoDateTime(" + v + ")";
        return v;
    }

    /** SDK → Salvo, from a non-null expression [v]. */
    private String ktFromSdk(Shape t, String v, int d) {
        if (t instanceof EnumShape) return v + ".value";
        if (t instanceof StructureShape s) return "fromSdk" + salvoType(s) + "(" + v + ")";
        if (t instanceof ListShape l) {
            return v + ".map { e" + d + " -> " + ktFromSdk(target(l.getMember()), "e" + d, d + 1) + " }";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? "k" + d + ".value" : "k" + d;
            String sortKey = key instanceof EnumShape ? "it.key.value" : "it.key";
            return "salvo.platform.core.map.canonicalMap(" + v + ".entries.sortedBy { " + sortKey + " }.map { (k" + d + ", v" + d + ") -> " + k + " to "
                    + ktFromSdk(target(m.getValue()), "v" + d, d + 1) + " })";
        }
        if (t instanceof BlobShape) return "salvo.SalvoBytes(" + v + ")";
        if (t instanceof TimestampShape) return "salvoInstant(" + v + ")";
        return v;
    }

    private static boolean ktNeedsConv(Shape t) {
        return !(t instanceof StringShape && !(t instanceof EnumShape)) && !isPrimitive(t);
    }

    private String ktReadField(MemberShape m, String src) {
        Shape t = target(m);
        String acc = src + "." + ktMember(m);
        if (present(m)) {
            return ktFromSdk(t, "(" + acc + " ?: error(\"missing required member " + m.getMemberName() + "\"))", 1);
        }
        return ktNeedsConv(t) ? acc + "?.let { " + ktFromSdk(t, "it", 1) + " }" : acc;
    }

    private String ktSetField(MemberShape m, String src) {
        Shape t = target(m);
        String field = src + "." + ktIdent(salvoField(m));
        if (present(m)) return ktToSdk(t, field, 1);
        return ktNeedsConv(t) ? field + "?.let { " + ktToSdk(t, "it", 1) + " }" : field;
    }

    private String kotlinGlue() {
        String effect = settings.effect();
        String pkg = "salvo.platform." + settings.module() + ".host";
        StringBuilder out = new StringBuilder();
        out.append("// Host implementation of `").append(settings.module()).append(".host`'s `Host").append(effect)
                .append("`,\n// over aws-sdk-kotlin (`").append(settings.kotlinPackage()).append("`) [platform-reply].\n//\n");
        out.append(header(""));
        out.append("@file:OptIn(aws.smithy.kotlin.runtime.InternalApi::class, aws.sdk.kotlin.runtime.InternalSdkApi::class)\n");
        out.append("@file:Suppress(\"DEPRECATION\", \"UNUSED_PARAMETER\", \"UNCHECKED_CAST\", \"USELESS_ELVIS\", \"UNNECESSARY_SAFE_CALL\")\n\n");
        out.append("package ").append(pkg).append("\n\nimport salvo.*\nimport salvo.aws.*\nimport salvo.")
                .append(settings.module()).append(".*\nimport salvo.").append(settings.module()).append(".host.*\n");
        out.append("import kotlinx.coroutines.launch\n");
        if (!streaming.isEmpty()) {
            out.append("import aws.smithy.kotlin.runtime.content.asByteStream\n");
            out.append("import aws.smithy.kotlin.runtime.content.toInputStream\n");
        }
        out.append('\n');

        out.append("// `threadsafe platform handler Host").append(effect).append("`: the SDK client is safe to share, and\n")
                .append("// each call is a coroutine on the handler's own scope, completing its reply there.\n");
        out.append("class Host").append(effect).append("(private val config: AwsConfig) : ").append(effect).append("Platform {\n");
        out.append("    private val scope = kotlinx.coroutines.CoroutineScope(\n")
                .append("        kotlinx.coroutines.SupervisorJob() + kotlinx.coroutines.Dispatchers.IO\n    )\n");
        out.append("    private val client = ").append(ktClient()).append(" {\n")
                .append("        region = this@Host").append(effect).append(".config.region.code\n")
                .append("        credentialsProvider = salvoCredentials(this@Host").append(effect).append(".config.credentials)\n")
                .append("        this@Host").append(effect).append(".config.endpoint?.let { endpointUrl = aws.smithy.kotlin.runtime.net.url.Url.parse(it) }\n")
                .append(settings.forcePathStyle()
                        ? "        forcePathStyle = this@Host" + effect + ".config.endpoint != null\n" : "")
                .append("    }\n")
                .append("\n    init {\n        salvoCloseWhenMainEnds(client)\n    }\n");
        for (OperationShape op : operations) out.append('\n').append(kotlinMember(op));
        out.append("}\n");

        out.append(kotlinCredentials());
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            StructureShape shape = model.expectShape(s.getKey(), StructureShape.class);
            if (!inputs.contains(s.getKey())) out.append('\n').append(kotlinFromSdk(shape, s.getValue()));
            if (!outputs.contains(s.getKey())) out.append('\n').append(kotlinToSdk(shape, s.getValue()));
        }
        out.append(kotlinFailure());
        if (usesTime) out.append(kotlinTimeHelpers());
        if (!streaming.isEmpty()) out.append(kotlinStreamHelpers());
        out.append("""

                private fun salvoAwsError(e: Throwable): AwsError {
                    val code = (e as? aws.smithy.kotlin.runtime.ServiceException)?.sdkErrorMetadata?.errorCode
                        ?: e::class.simpleName ?: "Exception"
                    return AwsError(code = code, message = e.message ?: e.toString())
                }
                """);
        return out.toString();
    }

    private String kotlinMember(OperationShape op) {
        String name = effectOp(op);
        MemberShape inStream = streamOf(input(op));
        MemberShape outStream = streamOf(output(op));
        StringBuilder out = new StringBuilder();
        out.append("    override fun ").append(ktIdent(name)).append('(');
        input(op).ifPresent(in -> out.append("input: ").append(structs.get(in.getId())).append(", "));
        out.append("reply: salvo.SalvoReply) {\n");
        out.append("        val host = reply.hosted()\n");
        if (inStream != null) {
            // [stream-table] Taken out of the host's table now — the token was
            // given up with the input; a foreign handle traps [stream-provider].
            out.append("        val body = SalvoStreams.takeIn(input.").append(ktIdent(salvoField(inStream)))
                    .append(".handle)\n");
            out.append("        val length = input.").append(ktIdent(salvoField(lengthOf(input(op).get()))))
                    .append('\n');
        }
        String okType = output(op).map(o -> structs.get(o.getId())).orElse("Unit");
        String errType = "Union2<" + errorStruct() + ", AwsError>";
        String answerType = "Union2<" + okType + ", salvo.core.checked.Checked<" + errType + ">>";
        String obj = opObject(op);
        out.append("        scope.launch {\n");
        String request = input(op).map(in -> "toSdk" + structs.get(in.getId()) + "(input"
                + (inStream != null ? ", upload.asByteStream(length)" : "") + ")").orElse("");
        if (inStream != null) {
            // No length, no request: the body is released unread (user
            // decision 2026-09-30 — never buffer to find it out).
            out.append("            if (length == null || length < 0) {\n")
                    .append("                body.closeInput()\n")
                    .append("                val failed: ").append(answerType)
                    .append(" = ").append(obj).append(".err(salvo.core.checked.Checked(").append(ktFailure("AwsError"))
                    .append("(AwsError(code = \"MissingContentLength\", message = \"")
                    .append(missingLengthText(op)).append("\"))))\n")
                    .append("                host.send(failed)\n")
                    .append("                return@launch\n            }\n");
            out.append("            val upload = SalvoUpload(body, length)\n");
        }
        String call = "client." + NamingKt.defaultName(op) + "(" + request + ")";
        if (outStream != null) {
            // The SDK's body lives only inside the response block, so the
            // block answers the reply itself and then waits for the program
            // to close the stream it was handed.
            out.append("            var sent = false\n");
            out.append("            val answer: ").append(answerType).append("? = try {\n");
            out.append("                ").append(call).append(" { response ->\n")
                    .append("                    val closed = kotlinx.coroutines.CompletableDeferred<Unit>()\n")
                    .append("                    val handle = salvoRegisterBody(\"").append(bodySource(op))
                    .append("\", response.").append(ktMember(outStream)).append(", closed)\n")
                    .append("                    sent = true\n")
                    .append("                    val ok: ").append(answerType).append(" = ").append(obj).append(".ok(fromSdk")
                    .append(structs.get(output(op).get().getId())).append("(response, handle))\n")
                    .append("                    host.send(ok)\n")
                    .append("                    closed.await()\n                }\n")
                    .append("                null\n");
        } else {
            out.append("            val answer: ").append(answerType).append(" = try {\n");
            if (output(op).isPresent()) {
                out.append("                ").append(obj).append(".ok(fromSdk").append(structs.get(output(op).get().getId())).append('(')
                        .append(call).append("))\n");
            } else {
                out.append("                ").append(call).append("\n                ").append(obj).append(".ok(Unit)\n");
            }
        }
        out.append("            } catch (e: aws.smithy.kotlin.runtime.ServiceException) {\n")
                .append("                ").append(obj).append(".err(salvo.core.checked.Checked(").append(ktFailure(errorStruct()))
                .append("(").append(kotlinExtra(op, "salvoFailure(e)"))
                .append(")))\n");
        out.append("            } catch (e: Exception) {\n")
                .append("                ").append(obj).append(".err(salvo.core.checked.Checked(").append(ktFailure("AwsError"))
                .append("(salvoAwsError(e))))\n            }\n");
        if (outStream != null) {
            out.append("            if (!sent && answer != null) host.send(answer)\n        }\n    }\n");
        } else if (inStream != null) {
            // A body that failed, or did not match its length, is the answer
            // whatever the service said about the bytes it did get.
            out.append("            val problem = upload.finish()\n")
                    .append("            body.closeInput()\n")
                    .append("            val result: ").append(answerType).append(" = problem?.let {\n")
                    .append("                ").append(obj).append(".err(salvo.core.checked.Checked(").append(ktFailure("AwsError"))
                    .append("(AwsError(code = \"StreamFailed\", message = it))))\n")
                    .append("            } ?: answer\n")
                    .append("            host.send(result)\n        }\n    }\n");
        } else {
            out.append("            host.send(answer)\n        }\n    }\n");
        }
        return out.toString();
    }

    /** [rustFailure]'s counterpart: a service exception as the service's error struct. */
    private String kotlinFailure() {
        String es = errorStruct();
        StringBuilder out = new StringBuilder();
        out.append("\n/** A code as the model names it (see the Rust glue's `salvo_code`). */\n");
        out.append("private fun salvoCode(code: String): String = when (val c = code.substringAfterLast('#')) {\n");
        for (Map.Entry<String, String> c : wireCodes().entrySet()) {
            out.append("    \"").append(c.getKey()).append("\" -> \"").append(c.getValue()).append("\"\n");
        }
        out.append("    else -> c\n}\n\n");
        out.append("private fun salvoFailure(e: aws.smithy.kotlin.runtime.ServiceException): ").append(es).append(" {\n")
                .append("    val meta = e.sdkErrorMetadata\n")
                .append("    val response = meta.protocolResponse as? aws.smithy.kotlin.runtime.http.response.HttpResponse\n")
                .append("    return ").append(es).append("(\n")
                .append("        code = salvoCode(meta.errorCode ?: \"Unknown\"),\n")
                .append("        message = meta.errorMessage ?: \"\",\n")
                .append("        status = response?.status?.value ?: 0,\n")
                .append("        requestId = meta.requestId,\n    )\n}\n");
        return out.toString();
    }

    /** [base] with the fields one operation's errors carry, where the exception is that error. */
    private String kotlinExtra(OperationShape op, String base) {
        StringBuilder fields = new StringBuilder();
        String out = base;
        for (ShapeId err : op.getErrors(service)) {
            StructureShape es = errorShapes.get(err.getName());
            List<MemberShape> extra = members(es).stream()
                    .filter(m -> !m.getMemberName().equalsIgnoreCase("message")).toList();
            if (extra.isEmpty()) continue;
            List<String> sets = new ArrayList<>();
            for (MemberShape m : extra) {
                sets.add(ktIdent(salvoField(m)) + " = " + ktReadField(m, "e"));
            }
            out = "(" + out + ").let { if (e is " + ktModel() + "." + ktSdkName(err) + ") it.copy("
                    + String.join(", ", sets) + ") else it }";
        }
        return out;
    }

    private String kotlinFromSdk(StructureShape s, String name) {
        StringBuilder out = new StringBuilder();
        MemberShape body = streaming.get(s.getId());
        String sdk = s.hasTrait("smithy.api#error") ? ktModel() + "." + ktSdkName(s.getId()) : ktSdkStruct(s.getId());
        out.append("private fun fromSdk").append(name).append("(v: ").append(sdk)
                .append(body != null ? ", bodyHandle: Long" : "").append("): ").append(name)
                .append(" = ").append(name).append('(');
        List<MemberShape> members = new ArrayList<>(members(s));
        if (members.isEmpty()) return out.append(")\n").toString();
        out.append('\n');
        for (MemberShape m : members) {
            String value = m.equals(body) ? "salvo.stream.InStream(bodyHandle)" : ktReadField(m, "v");
            out.append("    ").append(ktIdent(salvoField(m))).append(" = ").append(value).append(",\n");
        }
        out.append(")\n");
        return out.toString();
    }

    private String kotlinToSdk(StructureShape s, String name) {
        String sdk = ktSdkStruct(s.getId());
        MemberShape body = streaming.get(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("private fun toSdk").append(name).append("(v: ").append(name)
                .append(body != null ? ", bodyStream: aws.smithy.kotlin.runtime.content.ByteStream" : "").append("): ").append(sdk)
                .append(" = ").append(sdk).append(" {\n");
        for (MemberShape m : members(s)) {
            String value = m.equals(body)
                    ? "bodyStream" : ktSetField(m, "v");
            out.append("    ").append(ktMember(m)).append(" = ").append(value).append('\n');
        }
        out.append("}\n");
        return out.toString();
    }

    private String kotlinTimeHelpers() {
        return """

                /** A Smithy timestamp as a `time.Instant`: nanoseconds since the epoch. */
                private fun salvoInstant(t: aws.smithy.kotlin.runtime.time.Instant): salvo.time.Instant =
                    salvo.time.Instant(nanos = t.epochSeconds * 1_000_000_000L + t.nanosecondsOfSecond)

                /** A `time.Instant` as a Smithy timestamp. */
                private fun salvoDateTime(at: salvo.time.Instant): aws.smithy.kotlin.runtime.time.Instant =
                    aws.smithy.kotlin.runtime.time.Instant.fromEpochSeconds(
                        Math.floorDiv(at.nanos, 1_000_000_000L),
                        Math.floorMod(at.nanos, 1_000_000_000L).toInt(),
                    )
                """;
    }

    private String kotlinStreamHelpers() {
        return """

                /**
                 * [stream-table] Registers a response body in the host's stream table,
                 * answering its handle. [closed] completes when the program closes the
                 * stream — or reads it to its end — which is what lets the SDK's response
                 * block return and release the connection.
                 */
                private fun salvoRegisterBody(
                    source: String,
                    body: aws.smithy.kotlin.runtime.content.ByteStream?,
                    closed: kotlinx.coroutines.CompletableDeferred<Unit>,
                ): Long {
                    val input = body?.toInputStream()
                        ?: java.io.ByteArrayInputStream(ByteArray(0))
                    return SalvoStreams.registerIn(source, SalvoBody(input, closed), 0)
                }

                private class SalvoBody(
                    input: java.io.InputStream,
                    private val closed: kotlinx.coroutines.CompletableDeferred<Unit>,
                ) : java.io.FilterInputStream(input) {
                    override fun close() {
                        try {
                            super.close()
                        } finally {
                            closed.complete(Unit)
                        }
                    }
                }

                /**
                 * [stream-table] A request body streamed out of the host's table as the SDK
                 * reads it: exactly [length] bytes, since the SDKs sign and checksum a body
                 * of known length. A stream that ends early, or runs on past [length], fails
                 * the read that finds it, which fails the request; the first failure is
                 * kept in [problem]. Not retryable: a stream is read once.
                 */
                private class SalvoUpload(private val stream: SalvoIn, private val length: Long) : java.io.InputStream() {
                    @Volatile var problem: String? = null
                    private var left = length

                    override fun read(): Int {
                        val one = ByteArray(1)
                        return if (read(one, 0, 1) <= 0) -1 else one[0].toInt() and 0xff
                    }

                    override fun read(b: ByteArray, off: Int, len: Int): Int {
                        if (len == 0) return 0
                        if (left == 0L) return -1
                        val got = next(minOf(len.toLong(), left).toInt())
                        if (got.isEmpty()) {
                            fail("${stream.source}: ended after ${length - left} of the content_length of $length bytes")
                        }
                        // One byte past the end tells a stream longer than its length —
                        // checked *before* the last chunk is handed over, since the SDK
                        // stops reading at the length and would store a truncated object.
                        if (left - got.size == 0L && next(1).isNotEmpty()) {
                            fail("${stream.source}: longer than the content_length of $length bytes")
                        }
                        System.arraycopy(got, 0, b, off, got.size)
                        left -= got.size
                        return got.size
                    }

                    /** The upload's first failure, if any. */
                    fun finish(): String? = problem

                    private fun next(max: Int): ByteArray =
                        try {
                            synchronized(stream) { stream.readUpTo(max) }
                        } catch (e: SalvoFaultException) {
                            val f = e.fault
                            fail("${stream.source}: " + if (f is SalvoFault.Failed) f.message else "not valid UTF-8")
                        }

                    private fun fail(message: String): Nothing {
                        if (problem == null) problem = message
                        throw java.io.IOException(message)
                    }
                }
                """;
    }

    private String kotlinCredentials() {
        return """

                /** The SDK credentials provider an `AwsConfig`'s credentials name. */
                private fun salvoCredentials(
                    c: Union3<ProfileCredentials, EnvironmentCredentials, DefaultChain>,
                ): aws.smithy.kotlin.runtime.auth.awscredentials.CredentialsProvider = when (c) {
                    is Union3.U1<*, *, *> -> {
                        val p = c.value as ProfileCredentials
                        aws.sdk.kotlin.runtime.auth.credentials.ProfileCredentialsProvider(
                            profileName = p.profile,
                            configurationSource = aws.sdk.kotlin.runtime.config.profile.AwsConfigurationSource(
                                p.profile,
                                salvoExpandHome("~/.aws/config"),
                                salvoExpandHome(p.path),
                            ),
                        )
                    }
                    is Union3.U2<*, *, *> -> aws.sdk.kotlin.runtime.auth.credentials.EnvironmentCredentialsProvider()
                    is Union3.U3<*, *, *> -> aws.sdk.kotlin.runtime.auth.credentials.DefaultChainCredentialsProvider()
                }

                /**
                 * Closes [client] once the program's `main` thread has ended. The SDK's
                 * HTTP engine (OkHttp) keeps a non-daemon dispatcher thread alive for 60s
                 * after its last call, which would hold the JVM open that long after the
                 * program is done; a Rust program exits when `main` returns, and this
                 * makes the JVM do the same. Closing the client shuts the engine's
                 * executor down. A stopgap inside the glue: the runtime ending the
                 * process when `main` returns is on the roadmap, to be designed.
                 */
                private fun salvoCloseWhenMainEnds(client: AutoCloseable) {
                    val main = Thread.getAllStackTraces().keys.firstOrNull { it.name == "main" && it.threadGroup?.name == "main" }
                        ?: return
                    val closer = Thread {
                        main.join()
                        runCatching { client.close() }
                    }
                    closer.isDaemon = true
                    closer.name = "salvo-aws-close"
                    closer.start()
                }

                /** `~/` at the front of a path is the home directory, as a shell would read it. */
                private fun salvoExpandHome(path: String): String =
                    if (path.startsWith("~/")) (System.getenv("HOME") ?: System.getProperty("user.home")) + path.substring(1) else path
                """;
    }
}
