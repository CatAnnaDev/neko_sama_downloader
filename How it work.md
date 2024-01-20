# Start
- Setup args donner en cli + verification de leurs état 
- lancement de la recherche ou di direct download basé sur le -s en cli ou se qui est mis au moment de la question
- verification des paths pour ffmpeg ( mac linux ) installer dans l'OS et windows télécharger et mis dans le dossier utils
- on iter sur le ou les urls spécifier au moment de la recherche ou même di direct url 
- on start le chrome driver et on se co dessus en local host port 6969

# Process
- # La Base
- on créer le dossier tmp pour save les m3u8 après
- on fait un get_name_based_on_url pour chopper le bon nom pour save après
- si le dossier de save via le nom existe déjà on previens le user
- on vien scanner la main page
- on check si on est en direct url ou non

- # Le direct url
- on va sur la page direct avec le driver
- puis final process

- # La recherche
- on detect s'il y a plusieurs page ou non, si oui on choppe toute les urls relative pour la reconstruction après pour chaque page
- sinon on le fait mais que sur la 1er page trouvé
- on lance get_all_link_base_href pour reconstruire les url de chaque episodes par page 
- puis final process


- # final Process
- on entre dans l'iframe de jwplayer pour executé le js et sortir l'url du .m3u8
- on télécharge et save le m3u8 dans le dossier tmp
- on créer le dossier spécifique a la saison, Anime Download/La Langue/Le nom
- au moment de save le .m3u8 on le test pour savoir quelle url prendre dedans et indiqué a l'utilisateur les qualité vidéo dispo
- on commence pas la meilleur si elle marche on la garde sinon on le notifie et on test celle d'après etc
- et la on parse et save le bon m3u8 avec tous les liens de chaque .ts 
- on demande si l'utilisateur veux continuer le download avec un ou plusieurs épisode manquant ( s'il en manque au moins 1 )
- on shutdown chromedriver qui ne sert plus a rien ici
- on fait une verif sur le nombre de thread demander par le user que ça ne dépasse pas les capacité de sont cpu ou qu'on en spawn pas trop au vue du nombre d'épisode a download
- ici on iter sur le dossier tmp pour chopper toutes les paths vers les .m3u8, en plus de garder le path pour build la playlist vlc 
- on applique un sort sur le vec de path pour dl dans l'ordre pour les petit co au cas où
- on affiche la progress bar 
- on créer la threadpool et N thread pour start des process de ffmpeg pour download et save les vidéos en .mp4 dans le dossier spécifier
- uns fois tout télécharger donc viens faire la playlist vlc si on trouve au minima 2 episode
- on print le récap final du process

# Fin
- on print le temps global pris par l'app
- on re-kill chrome au cas où 