# Start
- Setup args donner en cli + verification de leurs état 
- lancement de la recherche ou du direct download basé sur le -s en cli ou se qui est mis au moment de la question
- verification des paths pour ffmpeg ( mac linux ) installer dans l'OS et windows télécharger et mis dans le dossier utils
- on iter sur le ou les urls spécifier au moment de la recherche ou même du direct url 
- on start le chrome driver et on se co dessus en localhost port 6969

# Process
- # La Base
- on créer le dossier tmp pour save les m3u8 après
- on fait un get_name_based_on_url pour chopper le bon nom pour save après
- si le dossier de save via le nom existe déjà on previens le user
- on check si on est en direct url ou non
- on vien scanner la main page

- # Le direct url
- on va sur la page direct avec le driver
- puis final process

- # La recherche
- on detect s'il y a plusieurs page ou non, si oui on choppe toute les urls relative pour la reconstruction après pour chaque page
- sinon on le fait mais que sur la 1er page trouvé
- on lance get_all_link_base_href pour reconstruire les url de chaque episodes par page 
- puis final process


- # final Process
- on iter sur chaque url du vec pour chopper le nom de l'épisode et rejoin l'ifamre après
- on entre dans l'iframe de jwplayer pour executé le js et sortir l'url Master du .m3u8
- on créer le dossier spécifique a la saison, Anime Download/La Langue/Le nom
- au moment de save le .m3u8 on le test pour savoir quelle url prendre dedans et indiqué a l'utilisateur les qualité vidéos dispo
- on commence par la meilleur si elle marche on la garde sinon on le notifie et on test celle d'après etc
- et la on parse et save le bon m3u8 avec tous les liens de chaque .ts 
- on demande a l'utilisateur s'il veux continuer le download avec un ou plusieurs épisode manquant ( s'il en manque au moins 1 )
- on shutdown chromedriver qui ne sert plus a rien ici
- on fait une verif sur le nombre de thread demander par le user que ça ne dépasse pas les capacité de sont cpu ou qu'on en spawn pas trop au vue du nombre d'épisode a download
- ici on iter sur le dossier tmp pour chopper toutes les paths vers les .m3u8, en plus de garder le path pour build la playlist vlc 
- on applique un sort sur le vec de path pour dl dans l'ordre pour les petites co au cas où
- on affiche la progress bar 
- on créer la threadpool avec N thread pour start des process de ffmpeg pour download et save les vidéos au format .mp4 dans le dossier spécifier
- une fois tout télécharger on viens faire la playlist vlc si on trouve au minima 2 episode
- on print le récap final du process

# Fin
- on print le temps global pris par l'app
- on re-kill chrome au cas où 

# Note
- avoir plus de 20 thread même avec une bonne co ne sert à rien
- le site limite la co de base en plus que pour chaque vidéo télécharger en réaliter c'est en moyenne 700~ mini vidéo ( la vidéo est en chunk c'est normal )
- ffmpeg arrive a timeout avec 20 thread parce qu'on spam trop le site donc on perd du temps au final même si ffmpeg gère le cas des timeout est arrive a chopper la vidéo
- on y gagne rien a en spawn plus que 20 limite on perd du temps
- ceci peux être vérifier si vous lancer un dl avec -v en cli 