use uroman::{Uroman, rom_format};

#[track_caller]
fn assert_romanizes_to_str(uroman: &Uroman, input: &str, lcode: Option<&str>, expected_str: &str) {
    let result = uroman.romanize_string::<rom_format::Str>(input, lcode);

    assert_eq!(result.to_string(), expected_str);
}

#[track_caller]
fn assert_romanizes_to_str_with_decode(
    uroman: &Uroman,
    input: &str,
    lcode: Option<&str>,
    expected_str: &str,
) {
    let result = uroman.romanize_escaped::<rom_format::Str>(input, lcode);

    assert_eq!(result.to_string(), expected_str);
}

#[test]
fn test_simple_romanization() {
    let uroman = Uroman::new();

    // Test a simple ASCII character
    assert_romanizes_to_str(&uroman, "A", None, "A");
    assert_romanizes_to_str(&uroman, "z", None, "z");

    // Test a character with a direct mapping from romanization-auto-table.txt (e.g., Greek Beta)
    // This assumes 'β' maps to 'b' in the auto-generated table
    assert_romanizes_to_str(&uroman, "β", None, "b");

    // Test a character with a direct mapping from UnicodeDataOverwrite.txt (e.g., Egyptian Hieroglyph)
    // This assumes '𓍧' maps to '600' in the overwrite table
    assert_romanizes_to_str(&uroman, "𓍧", None, "600");
}

#[test]
fn test_romanize_with_decode_unicode_escapes() {
    let uroman = Uroman::new();

    assert_romanizes_to_str_with_decode(&uroman, "fran\\xE7ais", Some("fra"), "fransais");

    // `Р` (U+0420), `у` (U+0443), `с` (U+0441), `к` (U+043A), `и` (U+0438), `й` (U+0439)
    let russian_escaped = "\\u0420\\u0443\\u0441\\u0441\\u043A\\u0438\\u0439";
    assert_romanizes_to_str_with_decode(&uroman, russian_escaped, Some("rus"), "Russky");

    // `你` (U+4F60), `好` (U+597D)
    assert_romanizes_to_str_with_decode(&uroman, "\\u4F60\\u597D", Some("zho"), "nihao");

    // emoji: "😀" (U+1F600)
    assert_romanizes_to_str_with_decode(&uroman, "\\U0001F600", None, "😀");

    assert_romanizes_to_str_with_decode(&uroman, "H\\x45LLO", None, "H\\x45LLO");

    assert_romanizes_to_str(&uroman, "fran\\xE7ais", Some("fra"), "fran\\xE7ais");
}

#[test]
fn test_ascii_passthrough() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "Hello World!", None, "Hello World!");
    assert_romanizes_to_str(&uroman, "12345", None, "12345");
    assert_romanizes_to_str(&uroman, "!@#$%^&*()", None, "!@#$%^&*()");
}

#[test]
fn test_kanji_number() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "六万五百三", None, "60503");
    assert_romanizes_to_str(&uroman, "二千万四十二", None, "20000042");
    assert_romanizes_to_str(&uroman, "八億五万一千二", None, "800051002");
}

#[test]
fn test_fractions() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "½", None, "1/2");
    assert_romanizes_to_str(&uroman, "¼", None, "1/4");
    assert_romanizes_to_str(&uroman, "¾", None, "3/4");
    assert_romanizes_to_str(&uroman, "23½", None, "23 1/2");
    assert_romanizes_to_str(&uroman, "1¼", None, "1 1/4");
    assert_romanizes_to_str(&uroman, "abc½", None, "abc1/2");
    assert_romanizes_to_str(&uroman, "½¼", None, "1/2 1/4");
}

#[test]
fn test_chinese_fractions_and_percentages() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "百分之一", None, "1%");
    assert_romanizes_to_str(&uroman, "百分之五", None, "5%");

    assert_romanizes_to_str(&uroman, "十分之一", None, "1/10");
    assert_romanizes_to_str(&uroman, "三分之二", None, "2/3");

    assert_romanizes_to_str(&uroman, "零分之五", None, "0fenzhi5");
    assert_romanizes_to_str(
        &uroman,
        "今年的增长率是百分之多少？一些分析师认为会更高。",
        None,
        "jinniandezengzhanglushibaifenzhiduoshao? 1xiefenxishirenweihuigenggao. ",
    );
}

#[test]
fn test_tibetan_edge_cases() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "བཟང", None, "bzang");
    assert_romanizes_to_str(&uroman, "འ", None, "'a");
    assert_romanizes_to_str(&uroman, "ཉིན", None, "nyin");
    assert_romanizes_to_str(&uroman, "འདིའི་", None, "'di'i·");
    assert_romanizes_to_str(&uroman, "འདིའི་", None, "'di'i·");
    assert_romanizes_to_str(&uroman, "འི", None, "i");
    assert_romanizes_to_str(&uroman, "འཁྲིད", None, "'khrid");
    assert_romanizes_to_str(&uroman, "ངའི་ཕའི་དཔེ་དེབ།", None, "nga'i·pha'i·dpe·deb,");
    assert_romanizes_to_str(&uroman, "བསྒྲུབས", None, "bsgrubs");
    assert_romanizes_to_str(&uroman, "ཨ་མདོ", None, "a·mdo");
    assert_romanizes_to_str(&uroman, "འེ", None, "'e");
}

#[test]
fn test_robustness_and_complex_fallbacks() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "百分之", None, "baifenzhi");
    assert_romanizes_to_str(&uroman, "零分之", None, "0fenzhi");
    assert_romanizes_to_str(&uroman, "十分之泰", None, "10fenzhitai");

    assert_romanizes_to_str(&uroman, "分之", None, "fenzhi");
    assert_romanizes_to_str(&uroman, "零分之½ไม่มี-๑๒๓%", None, "0fenzhi1/2maimii-123%");
    assert_romanizes_to_str(&uroman, "測試一百分之", None, "ceshi100fenzhi");
    assert_romanizes_to_str(&uroman, "100分之50", None, "50%");
}

#[test]
fn test_deu() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(&uroman, "Grüße", Some("deu"), "Gruesse");
    assert_romanizes_to_str(&uroman, "Schön", Some("deu"), "Schoen");
    assert_romanizes_to_str(&uroman, "Fußball", Some("deu"), "Fussball");
    assert_romanizes_to_str(
        &uroman,
        "Grüße aus Bordeaux",
        Some("deu"),
        "Gruesse aus Bordeaux",
    );
}

#[test]
fn test_tur() {
    let uroman = Uroman::new();

    assert_romanizes_to_str(
        &uroman,
        "İstanbul, Türkiye'de yer alan şehir ve ülkenin 81 ilinden biri.",
        Some("tur"),
        "Istanbul, Tuerkiye'de yer alan shehir ve uelkenin 81 ilinden biri.",
    );
}

#[test]
fn test_eng_braille() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "⠠⠺⠑⠀⠓⠕⠇⠙⠀⠘⠮⠀⠞⠗⠥⠹⠎⠀⠞⠕⠀⠆⠀⠎⠑⠇⠋⠤⠑⠧⠊⠙⠢⠞⠂⠀⠞⠀⠁⠇⠇⠀⠍⠑⠝⠀⠜⠑⠀⠉⠗⠂⠞⠫⠀⠑⠟⠥⠁⠇⠂⠀⠞⠀⠮⠽⠀⠜⠑⠀⠑⠝⠙⠪⠫⠀⠃⠽⠀⠸⠮⠀⠠⠉⠗⠑⠁⠞⠕⠗⠀⠾⠀⠉⠻⠞⠁⠔⠀⠥⠝⠁⠇⠊⠑⠝⠁⠃⠇⠑⠀⠠⠐⠗⠎⠂⠀⠞⠀⠁⠍⠰⠛⠀⠘⠮⠀⠜⠑⠀⠠⠇⠊⠋⠑⠂⠀⠠⠇⠊⠃⠻⠞⠽⠀⠯⠀⠮⠀⠏⠥⠗⠎⠥⠊⠞⠀⠷⠀⠠⠓⠁⠏⠏⠊⠰⠎⠲",
        Some("eng"),
        "We hold these truths to be self-evident, that all men are created equal, that they are endowed by their Creator with certain unalienable Rights, that among these are Life, Liberty and the pursuit of Happiness.",
    );
}

#[test]
fn test_ell() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "Το Λος Άντζελες (στα ισπανικά Los Angeles = Οι Άγγελοι) ή στην Αμερικανική αργκό L.A., ελ έι) είναι η δεύτερη μεγαλύτερη πόλη των Ηνωμένων Πολιτειών από άποψη πληθυσμού, καθώς και ένα από τα σημαντικότερα οικονομικά, πολιτιστικά επιστημονικά και ψυχαγωγικά κέντρα του κόσμου.",
        Some("ell"),
        "To Los Andzeles (sta ispanika Los Angeles = Oi Angeloi) e sten Amerikanike arngo L.A., el ei) einai e deutere megalutere pole ton Enomenon Politeion apo apopse plethysmou, kathos kai ena apo ta semandikotera oikonomika, politistika epistemonika kai psychagogika kendra tou kosmou.",
    );
}

#[test]
fn test_rus() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "Герма́ния (нем. Deutschland), официальное название — Федерати́вная Респу́блика Герма́ния (нем. Bundesrepublik Deutschland), ФРГ (нем. BRD) — государство в Западной Европе. Площадь территории — 357 021 км². Численность населения по переписи 2011 года — более 80 миллионов человек. [2][6].",
        Some("rus"),
        "Germaniya (nem. Deutschland), ofitsialnoye nazvaniye — Federativnaya Respublika Germaniya (nem. Bundesrepublik Deutschland), FRG (nem. BRD) — gosudarstvo v Zapadnoy Yevrope. Ploshchad territorii — 357 021 km². Chislennost naseleniya po perepisi 2011 goda — boleye 80 millionov chelovek. [2][6].",
    );
}

#[test]
fn test_ukr() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "Володи́мир Олекса́ндрович Зеле́нський (нар. 25 січня 1978, Кривий Ріг) — український державний діяч, політик, шоумен, актор, комік, режисер, продюсер та сценарист, шостий Президент України з 20 травня 2019 року.",
        Some("ukr"),
        "Volodymyr Oleksandrovych Zelensky (nar. 25 sichnya 1978, Kryvy Rih) — ukrayinsky derzhavny diyach, polityk, shoumen, aktor, komik, rezhyser, prodyuser ta stsenaryst, shosty Prezydent Ukrayiny z 20 travnya 2019 roku.",
    );
}

#[test]
fn test_srp() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "Сва људска бића рађају се слободна и једнака у достојанству и правима. Она су обдарена разумом и свешћу и треба једни према другима да поступају у духу братства.",
        Some("srp"),
        "Sva ljudska bitsha radjaju se slobodna i jednaka u dostojanstvu i pravima. Ona su obdarena razumom i sveshtshu i treba jedni prema drugima da postupaju u duhu bratstva.",
    );
}

#[test]
fn test_ara() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "كندا (بالإنجليزية: Canada) هي دولة في أمريكا الشمالية تتألف من 10 مقاطعات وثلاثة أقاليم. تقع في القسم الشمالي من القارة وتمتد من المحيط الأطلسي في الشرق إلى المحيط الهادئ في الغرب وتمتد شمالاً في المحيط المتجمد الشمالي. كندا هي البلد الثاني عالمياً من حيث المساحة الكلية. كما أن حدود كندا المشتركة مع الولايات المتحدة من الجنوب والشمال الغربي هي الأطول في العالم.",
        Some("ara"),
        "knda (balinjlyzya: Canada) hy dwla fy amryka alshmalya ttalf mn 10 mqat'at wthlatha aqalym. tq' fy alqsm alshmaly mn alqara wtmtd mn almhyt alatlsy fy alshrq ila almhyt alhadye fy alghrb wtmtd shmalan fy almhyt almtjmd alshmaly. knda hy albld althany 'almyan mn hyth almsaha alklya. kma an hdwd knda almshtrka m' alwlayat almthda mn aljnwb walshmal alghrby hy alatwl fy al'alm.",
    );
}

#[test]
fn test_fas() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "کالیفرنیا (به انگلیسی: California) ایالتی در غرب آمریکا بر کرانهٔ اقیانوس آرام است. مرکز آن ساکرامنتو و شهرهای مهم آن لس‌آنجلس، سن دیگو، سن خوزه و سان‌فرانسیسکو هستند.همچنین این ایالت پر جمعیت ترین ایالت امریکا است.",
        Some("fas"),
        "kalifrnia (be anglisi: California) ialti dr ghrb amrika br kraneye aqianvs aram ast. mrkz an sakramntv v shhrhai mhm an lsanjls, sn digv, sn khvze v sanfransiskv hstnd.hmchnin in ialt pr jmit trin ialt amrika ast.",
    );
}

#[test]
fn test_uig() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "﻿ئامېرىكا قوشما شتاتلىرى بولسا شىمالىي ئامېرىكاغا جايلاشقان بىر دۆلەت. ئۇنىڭ پايتەختى بولسا ۋاشىנגتون، ئەڭ چوڭ شەھىرى بولسا نيۇيورك شەھىرى. دۆلەت تىلى بولسا ئېנגلىزتىلى. ھازىرقى زۇڭتۇڭ باراك ئوباما. بۇ دۆلەت ئەسلىدە ئەنگىلىيەنىڭ مۇستەملىكىسى بولۇپ ۋاشىנגىتوننىڭ رەھپەرلىكىدە 1776 يىلى 7 ئاينىڭ 4 كۇنى مۇستەقىل بولغان، يەر مەيدانى 9 مىلىيون 826 مىڭ 630 كۋادىرات كلومېتىر، نوپۇسى 306 مىللىيون 142 مىڭ، بۇلارنىڭ ئاسساسلىق دىنى خرىستىئان دىنى.",
        Some("uig"),
        "amerika qoshma shtatliri bolsa shimaliy amerikagha jaylashqan bir doelet. uning paytexti bolsa washington, eng chong shehiri bolsa nyuyork shehiri. doelet tili bolsa engliztili. hazirqi zungtung barak obama. bu doelet eslide engiliening mustemlikisi bolup washingitonning rehperlikide 1776 yili 7 ayning 4 kuni musteqil bolghan, er meydani 9 miliyon 826 ming 630 kwadirat klometir, nopusi 306 milliyon 142 ming, bularning assasliq dini xristian dini.",
    );
}

#[test]
fn test_amh() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ኢትዮጵያ ከዓለም ሶስቱ ትልቅ የአብርሃም ሀይማኖቶች ጋር ታሪካዊ ግንኙነት አላት።",
        Some("amh"),
        "iteyopheyaa kaaalame sosetu teleqe yaaberehaame hayemaanotoche gaare taarikaawi genenyunate alaate.",
    );
}

#[test]
fn test_hin() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "कैलिफ़ोर्निया शब्द का पहला अर्थ था जो क्षेत्र जहाँ आज बाहा कैलिफ़ोर्निया प्रायद्वीप, नेवाडा, यूटा और एरिज़ोना, नया मेक्सिको, और वायोमिंग के कई विभाग स्थित हैं।",
        Some("hin"),
        "kailiforniyaa shabda kaa pahalaa artha thaa jo kssetra jahaam aaj baahaa kailiforniyaa praayadviip, nevaaddaa, yuuttaa aur erizonaa, nayaa meksiko, aur vaayomimg ke kaii vibhaag sthit haim.",
    );
}

#[test]
fn test_mar() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "लंडन (इंग्लिश: London ) हे इंग्लंडचे व युनायटेड किंग्डमचे राजधानीचे व सर्वात मोठे शहर तसेच युरोपियन संघामधील सर्वात मोठे महानगर क्षेत्र आहे.",
        Some("mar"),
        "lamddan (imglish: London ) he imglamddace va yunaayattedd kimgddamace raajadhaaniice va sarvaat motthe shahar tasec yuropiyan samghaamadhiil sarvaat motthe mahaanagar kssetra aahe.",
    );
}

#[test]
fn test_nep() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "यसको उचाइ समुन्द्र सतहबाट ८,८४८ मीटर (२९,०२८ फीट) छ। यो नेपालको सोलुखुम्बु जिल्लाको खुम्जुङ्ग गा. वि. स. मा पर्छ ।",
        Some("nep"),
        "yasako ucaai samundra satahabaatt 8,848 miittar (29,028 phiitt) cha. yo nepaalako solukhumbu jillaako khumjungga gaa. vi. sa. maa parcha .",
    );
}

#[test]
fn test_tam() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "தமிழ்நாடு (Tamil Nadu) இந்தியாவின் 29 மாநிலங்களில் ஒன்றாகும். தமிழ்நாடு, தமிழகம் என்றும் பரவலாக அழைக்கப்படுகிறது.",
        Some("tam"),
        "tamilnaadu (Tamil Nadu) intiyaavin 29 maanilangkalil onraakum. tamilnaadu, tamilakam enrum paravalaaka alaikkappadukiratu.",
    );
}

#[test]
fn test_mal() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ഇന്ത്യയുടെ തെക്കുപടിഞ്ഞാറെ അറ്റത്തുള്ള സംസ്ഥാനമാണ് കേരളം.",
        Some("mal"),
        "intyayutte tekkupattinynyaarre arrrrattulllla samsthaanamaann keerallam.",
    );
}

#[test]
fn test_ori() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        r###"ଓଡ଼ିଶା ଭାରତର ପୂର୍ବ ଉପକୂଳରେ ଥିବା ଏକ ପ୍ରଶାସନିକ ରାଜ୍ୟ । ଏହାର ଉତ୍ତର-ପୂର୍ବରେ ପଶ୍ଚିମବଙ୍ଗ, ଉତ୍ତରରେ ଝାଡ଼ଖଣ୍ଡ, ପଶ୍ଚିମ ଓ ଉତ୍ତର-ପଶ୍ଚିମରେ ଛତିଶଗଡ଼, ଦକ୍ଷିଣ ଓ ଦକ୍ଷିଣ-ପଶ୍ଚିମରେ ଆନ୍ଧ୍ରପ୍ରଦେଶ ଅବସ୍ଥିତ । ଏହା ଆୟତନ ହିସାବରେ ନବମ ଓ ଜନସଂଖ୍ୟା ହିସାବରେ ଏଗାରତମ ରାଜ୍ୟ । ଓଡ଼ିଆ ଭାଷା ରାଜ୍ୟର ସରକାରୀ ଭାଷା । ୨୦୦୧ ଜନଗଣନା ଅନୁସାରେ ରାଜ୍ୟର ପ୍ରାୟ ୩୩.୨ ନିୟୁତ ଲୋକ ଓଡ଼ିଆ ଭାଷା ବ୍ୟବହାର କରନ୍ତି । "###.trim(),
        Some("ori"),
        r###"oddishaa bhaaratara puurba upakuullare thibaa eka prashaasanika raajya . ehaara uttara-puurbare pashcimabangga, uttarare jhaaddakhanndda, pashcima o uttara-pashcimare chatishagadda, dakssinna o dakssinna-pashcimare aandhrapradesha abasthita . ehaa aayatana hisaabare nabama o janasamkhyaa hisaabare egaaratama raajya . oddiaa bhaassaa raajyara sarakaarii bhaassaa . 2001 janagannanaa anusaare raajyara praaya 33.2 niyuta loka oddiaa bhaassaa byabahaara karanti . "###.trim(),
    );
}

#[test]
fn test_zho() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "加拿大在一万四千年前即有原住民在此生活。",
        Some("zho"),
        "jianadazai14000nianqianjiyouyuanzhuminzaicishenghuo. ",
    );
}

#[test]
fn test_heb() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "כֹּל עוֹד בַּלֵּבָב פְּנִימָה נֶפֶשׁ יְהוּדִי הוֹמִיָּה וּלְפַאֲתֵי מִזְרָח, קָדִימָה, עַיִן לְצִיּוֹן צוֹפִיָּה, עוֹד לֹא אָבְדָה תִּקְוָתֵנוּ, הַתִּקְוָה בַּת שְׁנוֹת אַלְפַּיִם לִהְיוֹת עַם חָפְשִׁי בְּאַרְצֵנוּ, אֶרֶץ צִיּוֹן וִירוּשָׁלַיִם.",
        Some("heb"),
        "kol 'od balevav penimah nefesh yehudi homiyah ulefa'ate mizerach, qadimah, 'ayin letsiyon tsofiyah, 'od lo avedah tiqvatenu, hatiqvah bat shenot 'alepayim liheyot 'am chafeshiy be'aretsenu, erets tsiyon virushalayim.",
    );
}

#[test]
fn test_yid() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "דווקא איז אן העברעישער זשורנאל וואס באשרייבט די יידיש־שפראכיקע קולטור. עס איז דערשינען געווארן תמוז ה'תשס\"ז (יולי 2006).",
        Some("yid"),
        "duuka yz an hebreysher zhurnal was bashreybt dy eydysh-shfrachyke kultur. es yz dershynen gewarn smuz h'sshs\"z (yuly 2006).",
    );
}

#[test]
fn test_hye() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "Տալնոեի շրջան (ուկր.՝ Тальнівський район), շրջան Ուկրաինայի Չերկասիի մարզում։ Ստեղծվել է 1923 թվականին։ Վարչական կենտրոնը՝ Տալնոե։ Աշխարհագրությունը Շրջանի տարածքի մակերեսը կազմում է 917 կմ²։ Բնակչություն",
        Some("hye"),
        "Talnoei shrjan (ukr., Talnivsky raion), shrjan Ukrainayi Cherkasii marzum. Steghtsvel e 1923 tvakanin. Varchakan kentrone, Talnoe. Ashkharhagrutyune Shrjani taratski makerese kazmum e 917 km². Bnakchutyun",
    );
}

#[test]
// #[ignore = "skill issue"]
fn test_tai() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "มีประเทศอิสระ 2 ประเทศ คือ ซานมารีโนและนครรัฐวาติกัน เป็นดินแดนที่ล้อมรอบไปด้วยพื้นที่ของอิตาลี ในขณะที่เมืองกัมปีโอเนดีตาเลีย เป็นดินแดนส่วนแยกของอิตาลีที่ถูกล้อมรอบด้วยพื้นที่ประเทศสวิตเซอร์แลนด์",
        Some("tai"),
        "miiprathetitra 2 prathet khuee saanmaariinolaenkhanatwaatikan pendindaenthiilomroppaiduaiphueenthiikhongitaalii naiknathiimueangkampiionediitaalia pendindaensuanyaekkhongitaaliithiithuuklomropduaiphueenthiiprathetswitsoelaen",
    );
}

#[test]
fn test_generic_korean() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "북쪽에는 인도네시아와 동티모르, 파푸아 뉴기니, 북동쪽에는 솔로몬 제도와 바누아투, 누벨칼레도니, 그리고 남동쪽에는 뉴질랜드가 있다.",
        None,
        "bugjjogeneun indonesiawa dongtimoreu, papua nyugini, bugdongjjogeneun solromon jedowa banuatu, nubelkalredoni, geurigo namdongjjogeneun nyujilraendeuga issda.",
    );
}

#[test]
fn test_generic_kannada() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ಬಾ ಇಲ್ಲಿ ಸಂಭವಿಸು ಇಂದೆನ್ನ ಹೃದಯದಲಿ ನಿತ್ಯವೂ ಅವತರಿಪ ಸತ್ಯಾವತಾರ ಮಣ್ಣಾಗಿ ಮರವಾಗಿ ಮಿಗವಾಗಿ ಕಗವಾಗೀ... ಮಣ್ಣಾಗಿ ಮರವಾಗಿ ಮಿಗವಾಗಿ ಕಗವಾಗಿ ಭವ ಭವದಿ ಭತಿಸಿಹೇ ಭವತಿ ದೂರ ನಿತ್ಯವೂ ಅವತರಿಪ ಸತ್ಯಾವತಾರ || ಬಾ ಇಲ್ಲಿ ||",
        None,
        "baa illi sambhavisu imdenna hrdayadali nityavuu avataripa satyaavataara mannnnaagi maravaagi migavaagi kagavaagii... mannnnaagi maravaagi migavaagi kagavaagi bhava bhavadi bhatisihee bhavati duura nityavuu avataripa satyaavataara || baa illi ||",
    );
}

#[test]
fn test_generic_georgian() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ვეპხის ტყაოსანი შოთა რუსთაველი ღმერთსი შემვედრე, ნუთუ კვლა დამხსნას სოფლისა შრომასა, ცეცხლს, წყალსა და მიწასა, ჰაერთა თანა მრომასა; მომცנეს ფრთენი და აღვფრინდე, მივჰხვდე მას ჩემსა ნდომასა, დღისით და ღამით ვჰხედვიდე მზისა ელვათა კრთომაასა.",
        None,
        "vepxis tqaosani shota rustaveli ghmertsi shemvedre, nutu kvla damxsnas sophlisa shromasa, tsetsxls, tsqalsa da mitsasa, haerta tana mromasa; momtsnes phrteni da aghvphrinde, mivhxvde mas chemsa ndomasa, dghisit da ghamit vhxedvide mzisa elvata krtomaasa.",
    );
}

#[test]
fn test_generic_ogham() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "᚛ᚐᚅᚋ ᚋᚖᚂᚓᚌᚖᚋᚏᚔᚇ ᚋᚐᚉᚔ ᚍᚓᚉᚒᚋᚓᚅ᚜",
        None,
        "anm moilegoimrid maki vekumen",
    );
}

#[test]
fn test_generic_runic() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ᛁᚳ᛫ᛗᚨᚷ᛫ᚷᛚᚨᛋ᛫ᛖᚩᛏᚪᚾ᛫ᚩᚾᛞ᛫ᚻᛁᛏ᛫ᚾᛖ᛫ᚻᛖᚪᚱᛗᛁᚪᚧ᛫ᛗᛖ᛬",
        None,
        "ic mag glas eotan ond hit ne hearmiath me.",
    );
}

#[test]
fn test_generic_egyptian_hieroglyphs() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(&uroman, "𓊪𓏏𓍯𓃭𓐝𓇌𓋴", None, "ptolmys");
}

#[test]
fn test_generic_japanese() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(&uroman, "チェコスロバキア", None, "chekosurobakia");
}

#[test]
fn test_generic_tibetan() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(&uroman, "ལྷ་ས་གྲོང་ཁྱེར", None, "lha·sa·grong·khyer");
}

#[test]
fn test_generic_inuktitut() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ᓵᓕ ᓴᕕᐊᕐᔪᒃ ᐃᒻᒥᓂᒃ ᓂᓪᓕᕈᑎᖃᓲᖑᕗᖅ ᑕᐃᑦᓱᒪᓂᑕᑦᓴᔭᐅᓂᕋᕐᓱᓂ. ᐃᒻᒥᓂᓪᓗᑕᐅᖅ ᓂᓪᓕᕈᑎᖃᓱᖑᒻᒥᓱᓂ ᐅᓪᓗᒥᓂᑕᑦᓴᔭᐅᓂᕋᕐᓱᓂ.",
        None,
        "saali safiaryok imminik nillirotiqasoongofoq taitsomanitatsayaonirarsoni. imminillotaoq nillirotiqasongommisoni ollominitatsayaonirarsoni.",
    );
}

#[test]
fn test_generic_tifinagh() {
    let uroman = Uroman::new();
    assert_romanizes_to_str(
        &uroman,
        "ⴰⵎⴰⴳⵔⴰⴷ 1 ⴰⵔ ⴷ ⵜⵜⵍⴰⵍⴰⵏ ⵎⵉⴷⴷⵏ ⴳⴰⵏ ⵉⵍⴻⵍⵍⵉⵜⵏ ⵎⴳⴰⴷⴷⴰⵏ ⵖ ⵡⴰⴷⴷⵓⵔ ⴷ ⵉⵣⵔⴼⴰⵏ, ⵢⵉⵍⵉ ⴰⴽⵯ ⴷⴰⵔⵙⵏ ⵓⵏⵍⵍⵉ ⴷ ⵓⴼⵔⴰⴽ, ⵉⵍⵍⴰ ⴼⵍⵍⴰ ⵙⵏ ⴰⴷ ⵜⵜⵎⵢⴰⵡⴰⵙⵏ ⵏⴳⵔⴰⵜⵙⵏ ⵙ ⵜⴰⴳⵎⴰⵜ.",
        None,
        "amagrad 1 ar d ttlalan middn gan ilellitn mgaddan gh waddur d izrfan, yili ak darsn unlli d ufrak, illa flla sn ad ttmyawasn ngratsn s tagmat.",
    );
}
